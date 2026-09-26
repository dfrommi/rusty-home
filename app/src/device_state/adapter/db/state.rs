use anyhow::{Context as _, Result};
use moka::future::Cache;
use sqlx::PgPool;
use std::collections::HashSet;

use crate::{
    core::{id::ExternalId, time::DateTimeRange, timeseries::DataPoint},
    device_state::{DeviceStateId, DeviceStateValue},
    t,
};

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

    pub async fn save(&self, dp: DataPoint<DeviceStateValue>) -> Result<bool> {
        let fvalue = f64::from(&dp.value);
        let tag_id = self.get_tag_id(&dp.value.into()).await?;

        let result = sqlx::query!(
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
            tag_id as i32,
            fvalue,
            dp.timestamp.into_db()
        )
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    pub async fn get_latest_for_device(&self, id: &DeviceStateId) -> Result<Option<DataPoint<DeviceStateValue>>> {
        let tag_id = self.get_tag_id(id).await?;

        let row = sqlx::query!(
            r#"SELECT value as "value!", timestamp as "timestamp!"
                FROM thing_value
                WHERE tag_id = $1
                AND timestamp <= $2
                ORDER BY timestamp DESC
                LIMIT 1;"#,
            tag_id as i32,
            t!(now).into_db()
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| DataPoint {
            value: from_f64_value(*id, r.value),
            timestamp: r.timestamp.into(),
        }))
    }

    pub async fn get_all_data_points_in_range_ts_asc(
        &self,
        range: DateTimeRange,
    ) -> anyhow::Result<Vec<DataPoint<DeviceStateValue>>> {
        let recs = sqlx::query!(
            r#"SELECT
                v.value as "value!: f64",
                v.timestamp as "timestamp!",
                t.channel,
                t.name
            FROM thing_value_tag t
            JOIN LATERAL (
                (
                    SELECT tv.value, tv.timestamp
                    FROM thing_value tv
                    WHERE tv.tag_id = t.id
                      AND tv.timestamp >= $1
                      AND tv.timestamp <= $2
                )
                UNION ALL
                (
                    SELECT tv.value, tv.timestamp
                    FROM thing_value tv
                    WHERE tv.tag_id = t.id
                      AND tv.timestamp < $1
                    ORDER BY tv.timestamp DESC
                    LIMIT 1
                )
            ) v ON true
            ORDER BY v.timestamp asc;"#,
            range.start().into_db(),
            range.end().into_db(),
        )
        .fetch_all(&self.pool)
        .await?;

        let mut invalid_tags: HashSet<(String, String)> = HashSet::new();

        let dps: Vec<DataPoint<DeviceStateValue>> = recs
            .into_iter()
            .filter_map(|row| {
                let external_id = ExternalId::new(row.channel.as_str(), row.name.as_str());

                match DeviceStateId::try_from(external_id) {
                    Ok(target) => Some(DataPoint {
                        value: from_f64_value(target, row.value),
                        timestamp: row.timestamp.into(),
                    }),
                    Err(_) => {
                        invalid_tags.insert((row.channel.clone(), row.name.clone()));
                        None
                    }
                }
            })
            .collect();

        if !invalid_tags.is_empty() {
            tracing::debug!(
                "Found {} unsupported device-state tags in range [{}..{}]: {:?}",
                invalid_tags.len(),
                range.start(),
                range.end(),
                invalid_tags
            );
        }

        Ok(dps)
    }

    async fn get_tag_id(&self, id: &DeviceStateId) -> Result<i64> {
        self.tag_id_cache
            .try_get_with(*id, get_or_insert_tag_id_from_db(&self.pool, id))
            .await
            .map_err(|e| anyhow::anyhow!(e))
    }
}

fn from_f64_value(id: DeviceStateId, value: f64) -> DeviceStateValue {
    fn bool_of(f: f64) -> bool {
        f > f64::EPSILON
    }

    match id {
        DeviceStateId::AllergenIndex(id) => DeviceStateValue::AllergenIndex(id, value.into()),
        DeviceStateId::EnergySaving(id) => DeviceStateValue::EnergySaving(id, bool_of(value)),
        DeviceStateId::Opened(id) => DeviceStateValue::Opened(id, bool_of(value)),
        DeviceStateId::ParticulateMatter(id) => DeviceStateValue::ParticulateMatter(id, value.into()),
        DeviceStateId::PowerAvailable(id) => DeviceStateValue::PowerAvailable(id, bool_of(value)),
        DeviceStateId::Presence(id) => DeviceStateValue::Presence(id, bool_of(value)),
        DeviceStateId::CurrentPowerUsage(id) => DeviceStateValue::CurrentPowerUsage(id, value.into()),
        DeviceStateId::FanActivity(id) => DeviceStateValue::FanActivity(id, value.into()),
        DeviceStateId::HeatingDemand(id) => DeviceStateValue::HeatingDemand(id, value.into()),
        DeviceStateId::HeatingDemandLimit(id) => DeviceStateValue::HeatingDemandLimit(id, value.into()),
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
