use std::collections::HashMap;

use anyhow::{Context as _, Result};
use cached::proc_macro::cached;
use sqlx::{PgPool, postgres::types::PgInterval};

use crate::{
    core::{
        id::ExternalId,
        time::{DateTime, DateTimeRange, Duration},
        timeseries::DataPoint,
    },
    device_state::{DeviceStateId, DeviceStateValue, OfflineItem},
    t,
};

#[derive(Debug, Clone)]
pub struct DeviceStateRepository {
    pool: PgPool,
}

impl DeviceStateRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn save(&self, dp: DataPoint<DeviceStateValue>) -> Result<bool> {
        let fvalue = f64::from(&dp.value);
        let tag_id = get_tag_id(&self.pool, &dp.value.into()).await?;

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

    pub async fn get_latest_for_device(&self, id: &DeviceStateId) -> Result<DataPoint<DeviceStateValue>> {
        let tag_id = get_tag_id(&self.pool, id).await?;

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
        .fetch_one(&self.pool)
        .await?;

        Ok(DataPoint {
            value: from_f64_value(*id, row.value),
            timestamp: row.timestamp.into(),
        })
    }

    pub async fn get_latest_for_all_devices(&self) -> Result<HashMap<DeviceStateId, DataPoint<DeviceStateValue>>> {
        let rows = sqlx::query!(
            r#"SELECT DISTINCT ON (tag_id)
                       tag_id as "tag_id!: i64",
                       value as "value!",
                       timestamp as "timestamp!"
                FROM thing_value
                WHERE timestamp <= $1
                ORDER BY tag_id, timestamp DESC;"#,
            t!(now).into_db()
        )
        .fetch_all(&self.pool)
        .await?;

        let lookup = tag_id_lookup(&self.pool).await?;

        let mut result = HashMap::new();
        for row in rows.into_iter() {
            let Some(&id) = lookup.get(&row.tag_id) else {
                tracing::warn!("No DeviceStateId found for tag_id {}", row.tag_id);
                continue;
            };

            let dp = DataPoint {
                value: from_f64_value(id, row.value),
                timestamp: row.timestamp.into(),
            };

            result.insert(id, dp);
        }
        Ok(result)
    }

    pub async fn get_all_data_points_in_range(
        &self,
        range: DateTimeRange,
    ) -> anyhow::Result<Vec<DataPoint<DeviceStateValue>>> {
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

        let dps: Vec<DataPoint<DeviceStateValue>> = recs
            .into_iter()
            .filter_map(|row| {
                let external_id = ExternalId::new(row.channel.as_str(), row.name.as_str());

                match DeviceStateId::try_from(external_id) {
                    Ok(target) => Some(DataPoint {
                        value: from_f64_value(target, row.value),
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

    pub async fn update_device_availability(
        &self,
        device_id: &str,
        source: &str,
        last_seen: &DateTime,
        offline: bool,
    ) -> anyhow::Result<()> {
        sqlx::query!(
            r#"INSERT INTO item_availability (source, item, last_seen, marked_offline, considered_offline_after, entry_updated)
                VALUES ($1, $2, $3, $4, $5, $6)
                ON CONFLICT (source, item) DO UPDATE SET last_seen = $3, marked_offline = $4, entry_updated = $6"#,
            source,
            device_id,
            last_seen.into_db(),
            offline,
            //TODO should just work via chrono::Duration, but doesn't
            PgInterval::try_from(t!(1 hours).into_db()).unwrap(),
            t!(now).into_db(),
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_offline_items(&self) -> anyhow::Result<Vec<OfflineItem>> {
        let recs = sqlx::query!(
            r#"SELECT source, item, last_seen, marked_offline, considered_offline_after, entry_updated
                FROM item_availability"#
        )
        .fetch_all(&self.pool)
        .await?;

        let mut offline_items = vec![];

        for rec in recs.iter() {
            let considered_offline_after = convert_pginterval_to_duration(&rec.considered_offline_after);
            let duration = std::cmp::max(
                t!(now).elapsed_since(rec.last_seen.into()),
                t!(now).elapsed_since(rec.entry_updated.into()),
            );

            if rec.marked_offline || duration > considered_offline_after {
                offline_items.push(OfflineItem {
                    source: rec.source.clone(),
                    item: rec.item.clone(),
                    duration,
                });
            }
        }

        Ok(offline_items)
    }
}

fn convert_pginterval_to_duration(pg_interval: &PgInterval) -> Duration {
    let days_from_months = pg_interval.months * 30; // Rough estimation
    let total_days = days_from_months + pg_interval.days;

    let total_milliseconds = pg_interval.microseconds / 1_000;

    Duration::days(total_days as i64) + Duration::millis(total_milliseconds)
}

fn from_f64_value(id: DeviceStateId, value: f64) -> DeviceStateValue {
    fn bool_of(f: f64) -> bool {
        f > f64::EPSILON
    }

    match id {
        DeviceStateId::EnergySaving(id) => DeviceStateValue::EnergySaving(id, bool_of(value)),
        DeviceStateId::Opened(id) => DeviceStateValue::Opened(id, bool_of(value)),
        DeviceStateId::PowerAvailable(id) => DeviceStateValue::PowerAvailable(id, bool_of(value)),
        DeviceStateId::Presence(id) => DeviceStateValue::Presence(id, bool_of(value)),
        DeviceStateId::CurrentPowerUsage(id) => DeviceStateValue::CurrentPowerUsage(id, value.into()),
        DeviceStateId::FanActivity(id) => DeviceStateValue::FanActivity(id, value.into()),
        DeviceStateId::HeatingDemand(id) => DeviceStateValue::HeatingDemand(id, value.into()),
        DeviceStateId::LightLevel(id) => DeviceStateValue::LightLevel(id, value.into()),
        DeviceStateId::RawVendorValue(id) => DeviceStateValue::RawVendorValue(id, value.into()),
        DeviceStateId::RelativeHumidity(id) => DeviceStateValue::RelativeHumidity(id, value.into()),
        DeviceStateId::SetPoint(id) => DeviceStateValue::SetPoint(id, value.into()),
        DeviceStateId::Temperature(id) => DeviceStateValue::Temperature(id, value.into()),
        DeviceStateId::TotalEnergyConsumption(id) => DeviceStateValue::TotalEnergyConsumption(id, value.into()),
        DeviceStateId::TotalRadiatorConsumption(id) => DeviceStateValue::TotalRadiatorConsumption(id, value.into()),
        DeviceStateId::TotalWaterConsumption(id) => DeviceStateValue::TotalWaterConsumption(id, value.into()),
    }
}

async fn tag_id_lookup(pool: &PgPool) -> Result<HashMap<i64, DeviceStateId>> {
    let mut res = HashMap::new();

    for id in DeviceStateId::variants() {
        let tag_id = get_tag_id(pool, &id).await?;
        res.insert(tag_id, id);
    }

    Ok(res)
}

#[cached(result = true, key = "DeviceStateId", convert = r#"{ id.clone() }"#)]
async fn get_tag_id(db_pool: &PgPool, id: &DeviceStateId) -> Result<i64> {
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
