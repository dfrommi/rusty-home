use cached::proc_macro::cached;
use sqlx::PgPool;

use super::*;

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct DbChannelId {
    pub channel_name: &'static str,
    pub item_name: &'static str,
}

impl From<&ChannelValue> for DbChannelId {
    fn from(value: &ChannelValue) -> Self {
        match value {
            ChannelValue::Temperature(id, _) => id.into(),
            ChannelValue::RelativeHumidity(id, _) => id.into(),
            ChannelValue::Opened(id, _) => id.into(),
            ChannelValue::Powered(id, _) => id.into(),
            ChannelValue::CurrentPowerUsage(id, _) => id.into(),
            ChannelValue::TotalEnergyConsumption(id, _) => id.into(),
            ChannelValue::SetPoint(id, _) => id.into(),
            ChannelValue::HeatingDemand(id, _) => id.into(),
        }
    }
}

impl From<&ChannelValue> for f64 {
    fn from(val: &ChannelValue) -> Self {
        match val {
            ChannelValue::Temperature(_, v) => v.into(),
            ChannelValue::RelativeHumidity(_, v) => v.into(),
            ChannelValue::Opened(_, v) => v.into(),
            ChannelValue::Powered(_, v) => v.into(),
            ChannelValue::CurrentPowerUsage(_, v) => v.into(),
            ChannelValue::TotalEnergyConsumption(_, v) => v.into(),
            ChannelValue::SetPoint(_, v) => v.into(),
            ChannelValue::HeatingDemand(_, v) => v.into(),
        }
    }
}

impl From<&Temperature> for DbChannelId {
    fn from(val: &Temperature) -> Self {
        DbChannelId {
            channel_name: "temperature",
            item_name: val.into(),
        }
    }
}

impl From<&RelativeHumidity> for DbChannelId {
    fn from(val: &RelativeHumidity) -> Self {
        DbChannelId {
            channel_name: "relative_humidity",
            item_name: val.into(),
        }
    }
}

impl From<&Opened> for DbChannelId {
    fn from(val: &Opened) -> Self {
        DbChannelId {
            channel_name: "opened",
            item_name: val.into(),
        }
    }
}

impl From<&Powered> for DbChannelId {
    fn from(val: &Powered) -> Self {
        DbChannelId {
            channel_name: "powered",
            item_name: val.into(),
        }
    }
}

impl From<&CurrentPowerUsage> for DbChannelId {
    fn from(value: &CurrentPowerUsage) -> Self {
        DbChannelId {
            channel_name: "current_power_usage",
            item_name: value.into(),
        }
    }
}

impl From<&TotalEnergyConsumption> for DbChannelId {
    fn from(value: &TotalEnergyConsumption) -> Self {
        DbChannelId {
            channel_name: "total_energy_consumption",
            item_name: value.into(),
        }
    }
}

impl From<&SetPoint> for DbChannelId {
    fn from(value: &SetPoint) -> Self {
        DbChannelId {
            channel_name: "set_point",
            item_name: value.into(),
        }
    }
}
impl From<&HeatingDemand> for DbChannelId {
    fn from(value: &HeatingDemand) -> Self {
        DbChannelId {
            channel_name: "heating_demand",
            item_name: value.into(),
        }
    }
}

#[cached(
    result = true,
    key = "DbChannelId",
    convert = r#"{ channel_id.clone() }"#
)]
pub async fn get_tag_id(
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
