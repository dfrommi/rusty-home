use anyhow::Context;
use cached::proc_macro::cached;
use derive_more::derive::AsRef;
use sqlx::PgPool;
use support::ExternalId;

use super::*;

#[derive(Debug, Clone, PartialEq, sqlx::Type, AsRef)]
#[sqlx(transparent)]
pub struct DbValue(f64);

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

mod mapper {
    use support::unit::*;

    use super::{DbValue, unit::FanAirflow, unit::FanSpeed};

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
