use std::{fmt::Debug, sync::Arc};

use crate::{
    port::{DataPointAccess, TimeSeriesAccess},
    support::timeseries::{interpolate::Estimatable, TimeSeries},
};

use anyhow::{bail, Result};
use api::{
    get_tag_id,
    state::{db::DbValue, Channel},
};
use support::{t, time::DateTimeRange, DataFrame, DataPoint, ValueObject};

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
}

impl<T> DataPointAccess<T> for super::Database
where
    T: Into<Channel> + ValueObject + Debug + Clone,
    T::ValueType: From<DbValue> + Clone,
{
    #[tracing::instrument(skip(self))]
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
