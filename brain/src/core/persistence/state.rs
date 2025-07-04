use std::{fmt::Debug, sync::Arc};

use crate::{
    core::timeseries::{TimeSeries, interpolate::Estimatable},
    home::state::{Channel, ChannelValue},
    port::{DataPointAccess, TimeSeriesAccess},
};

use anyhow::{Context as _, Result, bail};
use cached::proc_macro::cached;
use derive_more::derive::AsRef;
use sqlx::PgPool;
use support::{
    DataFrame, DataPoint, ExternalId, ValueObject, t,
    time::{DateTime, DateTimeRange},
};

#[derive(Debug, Clone, PartialEq, sqlx::Type, AsRef)]
#[sqlx(transparent)]
pub struct DbValue(f64);

impl super::Database {
    fn ts_caching_range(&self) -> DateTimeRange {
        let now = t!(now);
        DateTimeRange::new(now - self.ts_cache_duration.clone(), now)
    }

    pub async fn preload_ts_cache(&self) -> anyhow::Result<()> {
        tracing::debug!("Start preloading cache");

        let tag_ids = get_all_tag_ids(&self.pool).await?;

        for tag_id in tag_ids {
            if let Err(e) = self.get_default_dataframe::<DbValue>(tag_id).await {
                tracing::error!(
                    "Error preloading timeseries cache for tag {}: {:?}",
                    tag_id,
                    e
                );
            }
        }

        tracing::debug!("Preloading cache done");
        Ok(())
    }

    pub async fn invalidate_ts_cache(&self, tag_id: i64) {
        tracing::debug!("Invalidating timeseries cache for tag {}", tag_id);
        self.ts_cache.invalidate(&tag_id).await;
    }

    pub async fn add_state(&self, value: &ChannelValue, timestamp: &DateTime) -> Result<()> {
        let tags_id = get_tag_id(&self.pool, value.into(), true).await?;

        let fvalue: DbValue = value.into();

        sqlx::query!(
            r#"WITH latest_value AS (
                SELECT value
                FROM thing_value
                WHERE tag_id = $1
                ORDER BY timestamp DESC, id DESC
                LIMIT 1
            )
            INSERT INTO thing_value (tag_id, value, timestamp)
            SELECT $1, $2, $3
            WHERE NOT EXISTS ( SELECT 1 FROM latest_value WHERE value = $2)"#,
            tags_id as i32,
            fvalue.as_ref(),
            timestamp.into_db()
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}

impl<T> DataPointAccess<T> for super::Database
where
    T: Into<Channel> + ValueObject + Debug + Clone,
    T::ValueType: From<DbValue> + Clone,
{
    async fn current_data_point(
        &self,
        item: T,
    ) -> Result<DataPoint<<T as ValueObject>::ValueType>> {
        let channel: Channel = item.into();
        let tag_id = get_tag_id(&self.pool, channel.clone(), false).await?;

        let df: DataFrame<T::ValueType> = self.get_default_dataframe(tag_id).await?;

        match df.prev_or_at(t!(now)) {
            Some(dp) => Ok(dp.clone()),
            None => anyhow::bail!("No data found"),
        }
    }
}

impl<T> TimeSeriesAccess<T> for super::Database
where
    T: Into<Channel> + Estimatable + Clone + Debug,
    T::Type: From<DbValue>,
{
    #[tracing::instrument(skip(self))]
    async fn series(&self, item: T, range: DateTimeRange) -> Result<TimeSeries<T>> {
        let channel: Channel = item.clone().into();
        let tag_id = get_tag_id(&self.pool, channel.clone(), false).await?;

        let df = self.get_default_dataframe(tag_id).await?;

        if range.start() < df.range().start() {
            tracing::warn!(
                "Timeseries out of cache range requested for item {:?} and range {}. Doing full query",
                tag_id,
                &range
            );

            let df = query_dataframe(&self.pool, tag_id, &range).await?;
            return TimeSeries::new(item, &From::from(&df), range);
        }

        TimeSeries::new(item, &df, range)
    }
}

impl super::Database {
    pub async fn get_all_data_points_in_range(
        &self,
        range: DateTimeRange,
    ) -> anyhow::Result<Vec<DataPoint<ChannelValue>>> {
        let recs = sqlx::query!(
            r#"SELECT 
                THING_VALUE.value as "value!: DbValue", 
                THING_VALUE.timestamp, 
                THING_VALUE_TAG.channel, 
                THING_VALUE_TAG.name
            FROM THING_VALUE
            JOIN THING_VALUE_TAG ON THING_VALUE_TAG.id = THING_VALUE.tag_id
            WHERE THING_VALUE.timestamp >= $1
            AND THING_VALUE.timestamp <= $2
            ORDER BY THING_VALUE.timestamp ASC"#,
            range.start().into_db(),
            range.end().into_db(),
        )
        .fetch_all(&self.pool)
        .await?;

        let dps: Vec<DataPoint<ChannelValue>> = recs
            .into_iter()
            .filter_map(|row| {
                let external_id = ExternalId::new(row.channel.as_str(), row.name.as_str());

                match Channel::try_from(external_id) {
                    Ok(target) => Some(DataPoint {
                        value: ChannelValue::from((target, row.value)),
                        timestamp: row.timestamp.into(),
                    }),
                    Err(e) => {
                        tracing::warn!(
                            "Received unsupported channel {}/{}: {:?}",
                            row.channel,
                            row.name,
                            e
                        );
                        None
                    }
                }
            })
            .collect();

        Ok(dps)
    }

    //try to return reference or at least avoid copy of entire dataframe
    async fn get_default_dataframe<T>(&self, tag_id: i64) -> anyhow::Result<DataFrame<T>>
    where
        T: From<DbValue> + Clone, //TODO remove clone, use ref
    {
        let df = self
            .ts_cache
            .try_get_with(tag_id, async {
                tracing::debug!(
                    "No cached data found for tag {}, fetching from database",
                    tag_id
                );

                query_dataframe(&self.pool, tag_id, &self.ts_caching_range())
                    .await
                    .map(Arc::new)
            })
            .await;

        match df {
            Ok(df) => Ok(df.map(|dp| From::from(dp.value.clone()))),
            Err(e) => bail!(
                "Error refreshing timeseries cache for tag {}: {:?}",
                tag_id,
                e
            ),
        }
    }
}

#[cached(result = true, key = "Channel", convert = r#"{ channel.clone() }"#)]
pub async fn get_tag_id(
    db_pool: &PgPool,
    channel: Channel,
    create_if_missing: bool,
) -> anyhow::Result<i64> {
    let id: &ExternalId = channel.as_ref();

    let tag_id = if create_if_missing {
        sqlx::query_scalar!(
            r#"WITH thing_value_tag_ins AS (
                INSERT INTO thing_value_tag (channel, name)
                VALUES ($1, $2)
                ON CONFLICT (channel, name)
                DO NOTHING
                RETURNING id
            )
            SELECT id as "id!"
            FROM thing_value_tag_ins
            UNION ALL
            SELECT id FROM thing_value_tag
                WHERE channel IS NOT DISTINCT FROM $1
                AND name IS NOT DISTINCT FROM $2
                LIMIT 1"#,
            id.ext_type(),
            id.ext_name()
        )
        .fetch_one(db_pool)
        .await
        .with_context(|| {
            format!(
                "Error getting or creating tag id for {}/{}",
                id.ext_type(),
                id.ext_name()
            )
        })
    } else {
        sqlx::query_scalar!(
            r#"SELECT id FROM thing_value_tag
                WHERE channel IS NOT DISTINCT FROM $1
                AND name IS NOT DISTINCT FROM $2
                LIMIT 1"#,
            id.ext_type(),
            id.ext_name()
        )
        .fetch_one(db_pool)
        .await
        .with_context(|| {
            format!(
                "Error getting tag id for {}/{}",
                id.ext_type(),
                id.ext_name()
            )
        })
    }?;

    Ok(tag_id as i64)
}

#[tracing::instrument(skip_all, fields(tag_id = tag_id))]
async fn query_dataframe(
    pool: &sqlx::PgPool,
    tag_id: i64,
    range: &DateTimeRange,
) -> anyhow::Result<DataFrame<DbValue>> {
    //TODO rewrite to max query
    let rec = sqlx::query!(
        r#"(SELECT value as "value!: DbValue", timestamp
              FROM THING_VALUE
              WHERE TAG_ID = $1
              AND timestamp >= $2
              AND timestamp <= $3
              AND timestamp <= $4)
            UNION ALL
            (SELECT value, timestamp
              FROM THING_VALUE
              WHERE TAG_ID = $1
              AND timestamp < $2
              AND timestamp <= $4
              ORDER BY timestamp DESC
              LIMIT 1)
            UNION ALL
            (SELECT value, timestamp
              FROM THING_VALUE
              WHERE TAG_ID = $1
              AND timestamp > $3
              AND timestamp <= $4
              ORDER BY timestamp ASC
              LIMIT 1)"#,
        tag_id as i32,
        range.start().into_db(),
        range.end().into_db(),
        t!(now).into_db(), //For timeshift in tests
    )
    .fetch_all(pool)
    .await?;

    let dps: Vec<DataPoint<DbValue>> = rec
        .into_iter()
        .map(|row| DataPoint {
            value: row.value,
            timestamp: row.timestamp.unwrap().into(),
        })
        .collect();

    DataFrame::new(dps)
}

async fn get_all_tag_ids(pool: &sqlx::PgPool) -> anyhow::Result<Vec<i64>> {
    let rec = sqlx::query!(r#"SELECT id FROM thing_value_tag"#)
        .fetch_all(pool)
        .await?;

    Ok(rec.into_iter().map(|row| row.id as i64).collect())
}

//TODO inline and make generic to only convert to/from f64
mod mapper {
    use support::unit::*;

    use crate::home::state::{FanAirflow, FanSpeed};

    use super::DbValue;

    impl From<&bool> for DbValue {
        fn from(value: &bool) -> Self {
            DbValue(if *value { 1.0 } else { 0.0 })
        }
    }

    impl From<DbValue> for bool {
        fn from(value: DbValue) -> Self {
            value.0 > 0.0
        }
    }

    impl From<&DegreeCelsius> for DbValue {
        fn from(value: &DegreeCelsius) -> Self {
            DbValue(*value.as_ref())
        }
    }

    impl From<DbValue> for DegreeCelsius {
        fn from(value: DbValue) -> Self {
            value.0.into()
        }
    }

    impl From<&Percent> for DbValue {
        fn from(value: &Percent) -> Self {
            DbValue(value.0)
        }
    }

    impl From<DbValue> for Percent {
        fn from(value: DbValue) -> Self {
            Self(value.0)
        }
    }

    impl From<&Watt> for DbValue {
        fn from(value: &Watt) -> Self {
            DbValue(value.0)
        }
    }

    impl From<DbValue> for Watt {
        fn from(value: DbValue) -> Self {
            Self(value.0)
        }
    }

    impl From<&KiloWattHours> for DbValue {
        fn from(value: &KiloWattHours) -> Self {
            DbValue(value.0)
        }
    }

    impl From<DbValue> for KiloWattHours {
        fn from(value: DbValue) -> Self {
            Self(value.0)
        }
    }

    impl From<&KiloCubicMeter> for DbValue {
        fn from(value: &KiloCubicMeter) -> Self {
            DbValue(value.0)
        }
    }

    impl From<DbValue> for KiloCubicMeter {
        fn from(value: DbValue) -> Self {
            Self(value.0)
        }
    }

    impl From<&HeatingUnit> for DbValue {
        fn from(value: &HeatingUnit) -> Self {
            DbValue(value.0)
        }
    }

    impl From<DbValue> for HeatingUnit {
        fn from(value: DbValue) -> Self {
            Self(value.0)
        }
    }

    impl From<&FanAirflow> for DbValue {
        fn from(value: &FanAirflow) -> Self {
            let f_value = match value {
                FanAirflow::Off => 0.0,
                FanAirflow::Forward(FanSpeed::Silent) => 1.0,
                FanAirflow::Forward(FanSpeed::Low) => 2.0,
                FanAirflow::Forward(FanSpeed::Medium) => 3.0,
                FanAirflow::Forward(FanSpeed::High) => 4.0,
                FanAirflow::Forward(FanSpeed::Turbo) => 5.0,
                FanAirflow::Reverse(FanSpeed::Silent) => -1.0,
                FanAirflow::Reverse(FanSpeed::Low) => -2.0,
                FanAirflow::Reverse(FanSpeed::Medium) => -3.0,
                FanAirflow::Reverse(FanSpeed::High) => -4.0,
                FanAirflow::Reverse(FanSpeed::Turbo) => -5.0,
            };

            DbValue(f_value)
        }
    }

    impl From<DbValue> for FanAirflow {
        fn from(value: DbValue) -> Self {
            if value.0 < -4.0 {
                FanAirflow::Reverse(FanSpeed::Turbo)
            } else if value.0 < -3.0 {
                FanAirflow::Reverse(FanSpeed::High)
            } else if value.0 < -2.0 {
                FanAirflow::Reverse(FanSpeed::Medium)
            } else if value.0 < -1.0 {
                FanAirflow::Reverse(FanSpeed::Low)
            } else if value.0 < 0.0 {
                FanAirflow::Reverse(FanSpeed::Silent)
            } else if value.0 > 4.0 {
                FanAirflow::Forward(FanSpeed::Turbo)
            } else if value.0 > 3.0 {
                FanAirflow::Forward(FanSpeed::High)
            } else if value.0 > 2.0 {
                FanAirflow::Forward(FanSpeed::Medium)
            } else if value.0 > 1.0 {
                FanAirflow::Forward(FanSpeed::Low)
            } else if value.0 > 0.0 {
                FanAirflow::Forward(FanSpeed::Silent)
            } else {
                FanAirflow::Off
            }
        }
    }
}
