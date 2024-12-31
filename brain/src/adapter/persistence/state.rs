use std::fmt::Debug;

use crate::{
    port::{DataPointAccess, TimeSeriesAccess},
    support::timeseries::{interpolate::Estimatable, TimeSeries},
};

use anyhow::Result;
use api::{
    get_tag_id,
    state::{db::DbValue, Channel, ChannelTypeInfo},
};
use sqlx::PgPool;
use support::{t, time::DateTimeRange, DataPoint};

impl super::Database {
    pub async fn invalidate_state(&self, tag_id: i64) {
        if let Some(cache) = &self.cache {
            tracing::debug!("Invalidating state cache for tag {}", tag_id);
            cache.invalidate(&tag_id).await;
        } else {
            tracing::debug!(
                "No state cache configured, cannot invalidate tag {}",
                tag_id
            );
        }
    }
}

impl<T> DataPointAccess<T> for super::Database
where
    T: Into<Channel> + ChannelTypeInfo + Debug + Clone,
    T::ValueType: From<DbValue>,
{
    #[tracing::instrument(skip(self))]
    async fn current_data_point(
        &self,
        item: T,
    ) -> Result<DataPoint<<T as ChannelTypeInfo>::ValueType>> {
        let channel = item.into();
        let tag_id = get_tag_id(&self.pool, channel.clone(), false).await?;

        if let Some(cache) = &self.cache {
            if let Some(cached) = cache.get(&tag_id).await {
                return Ok(cached.map_value(|v| v.clone().into()));
            }
        }

        tracing::debug!("Cache miss for item {:?}, fetching from database", channel);

        //TODO rewrite to max query
        let rec = sqlx::query!(
            r#"SELECT value as "value: DbValue", timestamp
            FROM THING_VALUE
            WHERE TAG_ID = $1
            AND timestamp <= $2
            ORDER BY timestamp DESC, id DESC
            LIMIT 1"#,
            tag_id as i32,
            t!(now).into_db(), //For timeshift in tests
        )
        .fetch_optional(&self.pool)
        .await?;

        match rec {
            Some(r) => {
                if let Some(cache) = &self.cache {
                    cache
                        .insert(tag_id, DataPoint::new(r.value.clone(), r.timestamp.into()))
                        .await;
                }
                let dp = DataPoint::new(r.value.into(), r.timestamp.into());
                Ok(dp)
            }
            None => anyhow::bail!("No data found"),
        }
    }
}

impl<DB, T> TimeSeriesAccess<T> for DB
where
    T: Into<Channel> + Estimatable + Clone + Debug,
    T::Type: From<DbValue>,
    DB: AsRef<PgPool>,
{
    #[tracing::instrument(skip(self))]
    async fn series(&self, item: T, range: DateTimeRange) -> Result<TimeSeries<T>> {
        //TODO no clone, use ref
        let tags_id = get_tag_id(self.as_ref(), item.clone().into(), false).await?;

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
            tags_id as i32,
            range.start().into_db(),
            range.end().into_db(),
            t!(now).into_db(), //For timeshift in tests
        )
        .fetch_all(self.as_ref())
        .await?;

        let dps: Vec<DataPoint<T::Type>> = rec
            .into_iter()
            .map(|row| DataPoint {
                value: T::Type::from(row.value),
                timestamp: row.timestamp.unwrap().into(),
            })
            .collect();

        TimeSeries::new(item, dps, range)
    }
}
