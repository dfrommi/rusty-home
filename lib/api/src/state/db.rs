use cached::proc_macro::cached;
use sqlx::PgPool;

use super::*;

#[cached(result = true, key = "Channel", convert = r#"{ channel.clone() }"#)]
pub async fn get_tag_id(
    db_pool: &PgPool,
    channel: Channel,
    create_if_missing: bool,
) -> std::result::Result<i32, sqlx::Error> {
    let channel_json = serde_json::to_value(channel).unwrap();
    let channel_name = channel_json
        .get("type")
        .expect("Channel requires 'type' in serialized JSON")
        .as_str()
        .unwrap();
    let device_name = channel_json
        .get("item")
        .expect("Channel requires 'item' in serialized JSON")
        .as_str()
        .unwrap();

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
        .bind(channel_name)
        .bind(device_name)
        .fetch_one(db_pool)
        .await?;

    Ok(rec.0)
}
