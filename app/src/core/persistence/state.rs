use std::fmt::Debug;

use crate::{
    core::timeseries::{DataFrame, DataPoint},
    home::state::{PersistentHomeState, PersistentHomeStateValue},
};

use crate::core::id::ExternalId;
use crate::core::time::{DateTime, DateTimeRange};
use anyhow::{Context as _, Result};
use cached::proc_macro::cached;
use sqlx::PgPool;

// Helper methods for cache management
impl super::Database {
    pub async fn get_all_tag_ids(&self) -> anyhow::Result<Vec<i64>> {
        let rec = sqlx::query!(r#"SELECT id FROM thing_value_tag"#)
            .fetch_all(&self.pool)
            .await?;

        Ok(rec.into_iter().map(|row| row.id as i64).collect())
    }

    pub async fn get_tag_id(&self, channel: PersistentHomeState, create_if_missing: bool) -> anyhow::Result<i64> {
        get_tag_id(&self.pool, channel, create_if_missing).await
    }

    #[tracing::instrument(skip_all, fields(tag_id = tag_id))]
    pub async fn get_dataframe_for_tag(&self, tag_id: i64, range: &DateTimeRange) -> anyhow::Result<DataFrame<f64>> {
        //TODO rewrite to max query
        let rec = sqlx::query!(
            r#"(SELECT value as "value!: f64", timestamp
              FROM THING_VALUE
              WHERE TAG_ID = $1
              AND timestamp >= $2
              AND timestamp <= $3)
            UNION ALL
            (SELECT value, timestamp
              FROM THING_VALUE
              WHERE TAG_ID = $1
              AND timestamp < $2
              ORDER BY timestamp DESC
              LIMIT 1)
            UNION ALL
            (SELECT value, timestamp
              FROM THING_VALUE
              WHERE TAG_ID = $1
              AND timestamp > $3
              ORDER BY timestamp ASC
              LIMIT 1)"#,
            tag_id as i32,
            range.start().into_db(),
            range.end().into_db()
        )
        .fetch_all(&self.pool)
        .await?;

        let dps: Vec<DataPoint<f64>> = rec
            .into_iter()
            .map(|row| DataPoint {
                value: row.value,
                timestamp: row.timestamp.unwrap().into(),
            })
            .collect();

        DataFrame::new(dps)
    }
}

// State Value Operations
// Methods for adding and retrieving state values from the database
impl super::Database {
    pub async fn add_state(&self, value: &PersistentHomeStateValue, timestamp: &DateTime) -> Result<()> {
        let tags_id = get_tag_id(&self.pool, value.into(), true).await?;
        let fvalue: f64 = value.value();

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
            fvalue,
            timestamp.into_db()
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_all_data_points_in_range(
        &self,
        range: DateTimeRange,
    ) -> anyhow::Result<Vec<DataPoint<PersistentHomeStateValue>>> {
        let recs = sqlx::query!(
            r#"SELECT 
                THING_VALUE.value as "value!: f64", 
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

        let dps: Vec<DataPoint<PersistentHomeStateValue>> = recs
            .into_iter()
            .filter_map(|row| {
                let external_id = ExternalId::new(row.channel.as_str(), row.name.as_str());

                match PersistentHomeState::try_from(external_id) {
                    Ok(target) => Some(DataPoint {
                        value: target.with_value(row.value),
                        timestamp: row.timestamp.into(),
                    }),
                    Err(e) => {
                        tracing::warn!("Received unsupported channel {}/{}: {:?}", row.channel, row.name, e);
                        None
                    }
                }
            })
            .collect();

        Ok(dps)
    }
}

#[cached(result = true, key = "PersistentHomeState", convert = r#"{ channel.clone() }"#)]
pub async fn get_tag_id(
    db_pool: &PgPool,
    channel: PersistentHomeState,
    create_if_missing: bool,
) -> anyhow::Result<i64> {
    let id = channel.ext_id();

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
            id.type_name(),
            id.variant_name()
        )
        .fetch_one(db_pool)
        .await
        .with_context(|| format!("Error getting or creating tag id for {}/{}", id.type_name(), id.variant_name()))
    } else {
        sqlx::query_scalar!(
            r#"SELECT id FROM thing_value_tag
                WHERE channel IS NOT DISTINCT FROM $1
                AND name IS NOT DISTINCT FROM $2
                LIMIT 1"#,
            id.type_name(),
            id.variant_name()
        )
        .fetch_one(db_pool)
        .await
        .with_context(|| format!("Error getting tag id for {}/{}", id.type_name(), id.variant_name()))
    }?;

    Ok(tag_id as i64)
}
