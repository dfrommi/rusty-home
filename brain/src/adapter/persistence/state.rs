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

impl<DB, T> DataPointAccess<T> for DB
where
    T: Into<Channel> + ChannelTypeInfo + Debug,
    T::ValueType: From<DbValue>,
    DB: AsRef<PgPool>,
{
    #[tracing::instrument(skip(self))]
    async fn current_data_point(&self, item: T) -> Result<DataPoint<T::ValueType>> {
        let tag_id = get_tag_id(self.as_ref(), item.into(), false).await?;

        //TODO rewrite to max query
        let rec = sqlx::query!(
            r#"SELECT value as "value: DbValue", timestamp
            FROM THING_VALUE
            WHERE TAG_ID = $1
            AND timestamp <= $2
            ORDER BY timestamp DESC, id DESC
            LIMIT 1"#,
            tag_id,
            t!(now).into_db(), //For timeshift in tests
        )
        .fetch_optional(self.as_ref())
        .await?;

        match rec {
            Some(r) => Ok(DataPoint {
                value: r.value.into(),
                timestamp: r.timestamp.into(),
            }),
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
            tags_id,
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
