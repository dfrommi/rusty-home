use std::f64;

use cached::proc_macro::cached;
use chrono::{DateTime, Utc};
use sqlx::{postgres::PgRow, PgPool, Row};

use crate::error::{Error, Result};

use super::{ChannelId, ChannelValue, DataPoint, DbChannelId};

pub async fn get_latest<'a, C: ChannelId>(
    db_pool: &PgPool,
    id: &'a C,
) -> Result<DataPoint<C::ValueType>>
where
    &'a C: Into<DbChannelId>,
    C::ValueType: From<f64>,
{
    let tag_id = get_tag_id(db_pool, id.into(), false).await?;

    //TODO rewrite to max query
    let rec: Option<PgRow> = sqlx::query(
        "SELECT value, timestamp
            FROM THING_VALUES
            WHERE TAG_ID = $1
            ORDER BY timestamp DESC
            LIMIT 1",
    )
    .bind(tag_id)
    .fetch_optional(db_pool)
    .await?;

    match rec {
        Some(r) => Ok(DataPoint {
            value: r.get::<f64, _>("value").into(),
            timestamp: r.get("timestamp"),
        }),
        None => Err(Error::NotFound),
    }
}

pub async fn get_covering<'a, C: ChannelId>(
    db_pool: &PgPool,
    id: &'a C,
    start: DateTime<Utc>,
) -> Result<Vec<DataPoint<C::ValueType>>>
where
    &'a C: Into<DbChannelId>,
    C::ValueType: From<f64>,
{
    let tags_id = get_tag_id(db_pool, id.into(), false).await?;

    //TODO rewrite to max query
    let rec = sqlx::query(
        "(SELECT value, timestamp
              FROM THING_VALUES
              WHERE TAG_ID = $1
              AND timestamp > $2)
            UNION ALL
            (SELECT value, timestamp
              FROM THING_VALUES
              WHERE TAG_ID = $1
              AND timestamp <= $2
              ORDER BY timestamp DESC
              LIMIT 1)",
    )
    .bind(tags_id)
    .bind(start)
    .fetch_all(db_pool)
    .await?;

    let dps: Vec<DataPoint<C::ValueType>> = rec
        .into_iter()
        .map(|row| DataPoint {
            value: C::ValueType::from(row.get("value")),
            timestamp: row.get("timestamp"),
        })
        .collect();

    Ok(dps)
}

pub async fn add_thing_value(
    db_pool: &PgPool,
    value: &ChannelValue,
    timestamp: &DateTime<Utc>,
) -> Result<()> {
    let tags_id = get_tag_id(db_pool, value.into(), true).await?;

    let fvalue: f64 = value.into();

    sqlx::query(
        "WITH latest_value AS (
                SELECT value
                FROM thing_values
                WHERE tag_id = $1
                ORDER BY timestamp DESC
                LIMIT 1
            )
            INSERT INTO thing_values (tag_id, value, timestamp)
            SELECT $1, $2, $3
            WHERE NOT EXISTS ( SELECT 1 FROM latest_value WHERE value = $2)",
    )
    .bind(tags_id)
    .bind(fvalue)
    .bind(timestamp)
    .execute(db_pool)
    .await?;

    //info!("Inserted new value: {:?}", event);

    Ok(())
}

#[cached(
    result = true,
    key = "DbChannelId",
    convert = r#"{ channel_id.clone() }"#
)]
async fn get_tag_id(
    db_pool: &PgPool,
    channel_id: DbChannelId,
    create_if_missing: bool,
) -> std::result::Result<i32, sqlx::Error> {
    let query = if create_if_missing {
        "WITH tags_ins AS (
                INSERT INTO tags (channel, name)
                VALUES ($1, $2)
                ON CONFLICT (channel, name)
                DO NOTHING
                RETURNING id
            )
            SELECT id FROM tags_ins
            UNION ALL
            SELECT id FROM tags
                WHERE channel IS NOT DISTINCT FROM $1
                AND name IS NOT DISTINCT FROM $2
                LIMIT 1"
    } else {
        "SELECT id FROM tags
                WHERE channel IS NOT DISTINCT FROM $1
                AND name IS NOT DISTINCT FROM $2
                LIMIT 1"
    };

    let rec: (i32,) = sqlx::query_as(query)
        .bind(channel_id.channel_name)
        .bind(channel_id.item_name)
        .fetch_one(db_pool)
        .await?;

    Ok(rec.0)
}
