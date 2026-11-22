use anyhow::{Context as _, Result};
use derive_more::Display;
use moka::future::Cache;
use sqlx::PgPool;

use crate::{
    core::{time::DateTime, timeseries::DataPoint},
    device_state::{DeviceStateId, DeviceStateValue},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Display)]
pub enum SaveOutcome {
    Inserted,
    Duplicate,
    Stale,
}

#[derive(Debug, Clone)]
pub struct PostgresDeviceStateRepository {
    pool: PgPool,
    tag_id_cache: Cache<DeviceStateId, i64>,
}

impl PostgresDeviceStateRepository {
    pub fn new(pool: PgPool) -> Self {
        Self {
            pool,
            tag_id_cache: Cache::builder().build(),
        }
    }

    pub async fn save(&self, dp: DataPoint<DeviceStateValue>) -> Result<SaveOutcome> {
        let fvalue = f64::from(&dp.value);
        let minimum_delta = dp.value.minimum_storage_delta().unwrap_or(0.0);
        let tag_id = self.get_tag_id(&dp.value.into()).await?;
        let timestamp = dp.timestamp.into_db();

        let result = sqlx::query!(
            r#"WITH latest_value AS (
                SELECT value, timestamp
                FROM thing_value
                WHERE tag_id = $1
                ORDER BY timestamp DESC, id DESC
                LIMIT 1
            ), inserted AS (
                INSERT INTO thing_value (tag_id, value, timestamp)
                SELECT $1, $2, $3
                WHERE NOT EXISTS (
                    SELECT 1
                    FROM latest_value
                    WHERE value = $2
                       OR timestamp > $3
                       OR abs(value - $2) < $4
                )
                RETURNING 1
            )
            SELECT
                EXISTS (SELECT 1 FROM inserted) AS "inserted!",
                EXISTS (SELECT 1 FROM latest_value WHERE timestamp > $3) AS "stale!""#,
            tag_id as i32,
            fvalue,
            timestamp,
            minimum_delta
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(match (result.inserted, result.stale) {
            (true, _) => SaveOutcome::Inserted,
            (false, true) => SaveOutcome::Stale,
            (false, false) => SaveOutcome::Duplicate,
        })
    }

    async fn get_tag_id(&self, id: &DeviceStateId) -> Result<i64> {
        self.tag_id_cache
            .try_get_with(*id, get_or_insert_tag_id_from_db(&self.pool, id))
            .await
            .map_err(|e| anyhow::anyhow!(e))
    }
}

pub(super) fn from_f64_value(id: DeviceStateId, value: f64) -> DeviceStateValue {
    fn bool_of(f: f64) -> bool {
        f > f64::EPSILON
    }

    match id {
        DeviceStateId::AllergenIndex(id) => DeviceStateValue::AllergenIndex(id, value.into()),
        DeviceStateId::EnergySaving(id) => DeviceStateValue::EnergySaving(id, bool_of(value)),
        DeviceStateId::CompletionTime(id) => {
            DeviceStateValue::CompletionTime(id, DateTime::from_unix_millis(value.round() as i64))
        }
        DeviceStateId::Opened(id) => DeviceStateValue::Opened(id, bool_of(value)),
        DeviceStateId::ParticulateMatter(id) => DeviceStateValue::ParticulateMatter(id, value.into()),
        DeviceStateId::PowerAvailable(id) => DeviceStateValue::PowerAvailable(id, bool_of(value)),
        DeviceStateId::Presence(id) => DeviceStateValue::Presence(id, bool_of(value)),
        DeviceStateId::CurrentPowerUsage(id) => DeviceStateValue::CurrentPowerUsage(id, value.into()),
        DeviceStateId::FanActivity(id) => DeviceStateValue::FanActivity(id, value.into()),
        DeviceStateId::HeatingDemand(id) => DeviceStateValue::HeatingDemand(id, value.into()),
        DeviceStateId::HeatingDemandLimit(id) => DeviceStateValue::HeatingDemandLimit(id, value.into()),
        DeviceStateId::IsRunning(id) => DeviceStateValue::IsRunning(id, bool_of(value)),
        DeviceStateId::LightLevel(id) => DeviceStateValue::LightLevel(id, value.into()),
        DeviceStateId::RelativeHumidity(id) => DeviceStateValue::RelativeHumidity(id, value.into()),
        DeviceStateId::SetPoint(id) => DeviceStateValue::SetPoint(id, value.into()),
        DeviceStateId::Temperature(id) => DeviceStateValue::Temperature(id, value.into()),
        DeviceStateId::TotalEnergyConsumption(id) => DeviceStateValue::TotalEnergyConsumption(id, value.into()),
        DeviceStateId::TotalRadiatorConsumption(id) => DeviceStateValue::TotalRadiatorConsumption(id, value.into()),
    }
}

async fn get_or_insert_tag_id_from_db(db_pool: &PgPool, id: &DeviceStateId) -> Result<i64> {
    let id = id.ext_id();

    let tag_id = sqlx::query_scalar!(
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
    .with_context(|| format!("Error getting or creating tag id for {}/{}", id.type_name(), id.variant_name()))?;

    Ok(tag_id as i64)
}
