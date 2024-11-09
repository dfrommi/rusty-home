use anyhow::Result;
use api::{
    get_tag_id,
    state::{db::DbValue, Channel, ChannelTypeInfo},
};
use chrono::{DateTime, Utc};

use super::{DataPoint, HomeApi};

pub trait StateRepository {
    async fn get_latest<'a, C: ChannelTypeInfo>(
        &self,
        id: &'a C,
    ) -> Result<DataPoint<C::ValueType>>
    where
        &'a C: Into<Channel>,
        C::ValueType: From<DbValue>;
    async fn get_covering<'a, C: ChannelTypeInfo>(
        &self,
        id: &'a C,
        start: DateTime<Utc>,
    ) -> Result<Vec<DataPoint<C::ValueType>>>
    where
        &'a C: Into<Channel>,
        C::ValueType: From<DbValue>;
}

impl StateRepository for HomeApi {
    async fn get_latest<'a, C: ChannelTypeInfo>(&self, id: &'a C) -> Result<DataPoint<C::ValueType>>
    where
        &'a C: Into<Channel>,
        C::ValueType: From<DbValue>,
    {
        let tag_id = get_tag_id(&self.db_pool, id.into(), false).await?;

        //TODO rewrite to max query
        let rec = sqlx::query!(
            r#"SELECT value as "value: DbValue", timestamp
            FROM THING_VALUES
            WHERE TAG_ID = $1
            ORDER BY timestamp DESC, id DESC
            LIMIT 1"#,
            tag_id
        )
        .fetch_optional(&self.db_pool)
        .await?;

        match rec {
            Some(r) => Ok(DataPoint {
                value: r.value.into(),
                timestamp: r.timestamp,
            }),
            None => anyhow::bail!("No data found"),
        }
    }

    async fn get_covering<'a, C: ChannelTypeInfo>(
        &self,
        id: &'a C,
        start: DateTime<Utc>,
    ) -> Result<Vec<DataPoint<C::ValueType>>>
    where
        &'a C: Into<Channel>,
        C::ValueType: From<DbValue>,
    {
        let tags_id = get_tag_id(&self.db_pool, id.into(), false).await?;

        //TODO rewrite to max query
        let rec = sqlx::query!(
            r#"(SELECT value as "value!: DbValue", timestamp as "timestamp!: DateTime<Utc>"
              FROM THING_VALUES
              WHERE TAG_ID = $1
              AND timestamp > $2)
            UNION ALL
            (SELECT value, timestamp
              FROM THING_VALUES
              WHERE TAG_ID = $1
              AND timestamp <= $2
              ORDER BY timestamp DESC
              LIMIT 1)"#,
            tags_id,
            start
        )
        .fetch_all(&self.db_pool)
        .await?;

        let dps: Vec<DataPoint<C::ValueType>> = rec
            .into_iter()
            .map(|row| DataPoint {
                value: C::ValueType::from(row.value),
                timestamp: row.timestamp,
            })
            .collect();

        Ok(dps)
    }
}
