use cached::proc_macro::cached;
use sqlx::PgPool;

use super::*;

#[derive(Debug, Clone, PartialEq, sqlx::Type)]
#[sqlx(transparent)]
pub struct DbValue(f64);

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

mod mapper {
    use super::value_type::*;
    use super::DbValue;

    impl From<&DegreeCelsius> for DbValue {
        fn from(value: &DegreeCelsius) -> Self {
            DbValue(value.0)
        }
    }

    impl From<DbValue> for DegreeCelsius {
        fn from(value: DbValue) -> Self {
            Self(value.0)
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

    impl From<&OpenedState> for DbValue {
        fn from(value: &OpenedState) -> Self {
            match value {
                OpenedState::Opened => DbValue(1.0),
                OpenedState::Closed => DbValue(0.0),
            }
        }
    }

    impl From<DbValue> for OpenedState {
        fn from(value: DbValue) -> Self {
            if value.0 > 0.0 {
                Self::Opened
            } else {
                Self::Closed
            }
        }
    }

    impl From<&PowerState> for DbValue {
        fn from(value: &PowerState) -> Self {
            match value {
                PowerState::On => DbValue(1.0),
                PowerState::Off => DbValue(0.0),
            }
        }
    }

    impl From<DbValue> for PowerState {
        fn from(value: DbValue) -> Self {
            if value.0 > 0.0 {
                Self::On
            } else {
                Self::Off
            }
        }
    }

    impl From<&PresentState> for DbValue {
        fn from(value: &PresentState) -> Self {
            match value {
                PresentState::Present => DbValue(1.0),
                PresentState::Absent => DbValue(0.0),
            }
        }
    }

    impl From<DbValue> for PresentState {
        fn from(value: DbValue) -> Self {
            if value.0 > 0.0 {
                Self::Present
            } else {
                Self::Absent
            }
        }
    }

    impl From<&UserControlledState> for DbValue {
        fn from(value: &UserControlledState) -> Self {
            match value {
                UserControlledState::User => DbValue(1.0),
                UserControlledState::System => DbValue(0.0),
            }
        }
    }

    impl From<DbValue> for UserControlledState {
        fn from(value: DbValue) -> Self {
            if value.0 > 0.0 {
                Self::System
            } else {
                Self::User
            }
        }
    }
}
