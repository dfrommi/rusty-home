use anyhow::Context;
use cached::proc_macro::cached;
use derive_more::derive::AsRef;
use sqlx::PgPool;

use super::*;

#[derive(Debug, Clone, PartialEq, sqlx::Type, AsRef)]
#[sqlx(transparent)]
pub struct DbValue(f64);

#[cached(result = true, key = "Channel", convert = r#"{ channel.clone() }"#)]
pub async fn get_tag_id(
    db_pool: &PgPool,
    channel: Channel,
    create_if_missing: bool,
) -> anyhow::Result<i32> {
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

    if create_if_missing {
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
            channel_name,
            device_name
        )
        .fetch_one(db_pool)
        .await
        .with_context(|| {
            format!(
                "Error getting or creating tag id for {}/{}",
                channel_name, device_name
            )
        })
    } else {
        sqlx::query_scalar!(
            r#"SELECT id FROM thing_value_tag
                WHERE channel IS NOT DISTINCT FROM $1
                AND name IS NOT DISTINCT FROM $2
                LIMIT 1"#,
            channel_name,
            device_name
        )
        .fetch_one(db_pool)
        .await
        .with_context(|| format!("Error getting tag id for {}/{}", channel_name, device_name))
    }
}

mod mapper {
    use support::unit::*;

    use super::DbValue;

    impl From<&bool> for DbValue {
        fn from(value: &bool) -> Self {
            DbValue(if *value { 1.0 } else { 0.0 })
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

    impl From<DbValue> for bool {
        fn from(value: DbValue) -> Self {
            value.0 > 0.0
        }
    }
}
