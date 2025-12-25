use anyhow::{Context as _, Result};
use moka::future::Cache;
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
    tag_id_cache: Cache<DeviceStateId, i64>,
}

impl DeviceStateRepository {
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

    pub async fn get_latest_for_device(&self, id: &DeviceStateId) -> Result<DataPoint<DeviceStateValue>> {
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
        .fetch_one(&self.pool)
        .await?;

        Ok(DataPoint {
            value: from_f64_value(*id, row.value),
            timestamp: row.timestamp.into(),
        })
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

    async fn get_tag_id(&self, id: &DeviceStateId) -> Result<i64> {
        self.tag_id_cache
            .try_get_with(*id, get_or_insert_tag_id_from_db(&self.pool, id))
            .await
            .map_err(|e| anyhow::anyhow!(e))
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
        DeviceStateId::RelativeHumidity(id) => DeviceStateValue::RelativeHumidity(id, value.into()),
        DeviceStateId::SetPoint(id) => DeviceStateValue::SetPoint(id, value.into()),
        DeviceStateId::Temperature(id) => DeviceStateValue::Temperature(id, value.into()),
        DeviceStateId::TotalEnergyConsumption(id) => DeviceStateValue::TotalEnergyConsumption(id, value.into()),
        DeviceStateId::TotalRadiatorConsumption(id) => DeviceStateValue::TotalRadiatorConsumption(id, value.into()),
        DeviceStateId::TotalWaterConsumption(id) => DeviceStateValue::TotalWaterConsumption(id, value.into()),
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

#[cfg(test)]
mod tests {
    use crate::{core::unit::DegreeCelsius, device_state::Temperature};

    use super::*;

    #[sqlx::test(migrations = "../migrations")]
    async fn test_get_all_data_points_in_range_ts_asc(pool: PgPool) -> anyhow::Result<()> {
        let repo = DeviceStateRepository::new(pool);
        prepare_test_data(&repo).await?;

        let dps = repo
            .get_all_data_points_in_range_ts_asc(DateTimeRange::since(t!(35 minutes ago)))
            .await?;

        assert_eq!(dps.len(), 4);
        assert_eq!(
            dps[0].value,
            DeviceStateValue::Temperature(Temperature::LivingRoom, DegreeCelsius(21.0))
        );
        assert_eq!(
            dps[1].value,
            DeviceStateValue::Temperature(Temperature::LivingRoom, DegreeCelsius(21.5))
        );
        assert_eq!(
            dps[2].value,
            DeviceStateValue::Temperature(Temperature::Bedroom, DegreeCelsius(19.0))
        );
        assert_eq!(
            dps[3].value,
            DeviceStateValue::Temperature(Temperature::LivingRoom, DegreeCelsius(22.0))
        );

        Ok(())
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn test_get_latest_for_device(pool: PgPool) -> anyhow::Result<()> {
        let repo = DeviceStateRepository::new(pool);
        prepare_test_data(&repo).await?;

        let dp = repo
            .get_latest_for_device(&DeviceStateId::Temperature(Temperature::LivingRoom))
            .await?;

        assert_eq!(
            dp.value,
            DeviceStateValue::Temperature(Temperature::LivingRoom, DegreeCelsius(22.0))
        );

        Ok(())
    }

    async fn prepare_test_data(repo: &DeviceStateRepository) -> anyhow::Result<()> {
        repo.save(DataPoint::new(
            DeviceStateValue::Temperature(Temperature::LivingRoom, DegreeCelsius(20.5)),
            t!(50 minutes ago),
        ))
        .await?;

        repo.save(DataPoint::new(
            DeviceStateValue::Temperature(Temperature::LivingRoom, DegreeCelsius(21.0)),
            t!(40 minutes ago),
        ))
        .await?;

        repo.save(DataPoint::new(
            DeviceStateValue::Temperature(Temperature::LivingRoom, DegreeCelsius(21.5)),
            t!(30 minutes ago),
        ))
        .await?;

        repo.save(DataPoint::new(
            DeviceStateValue::Temperature(Temperature::LivingRoom, DegreeCelsius(22.0)),
            t!(20 minutes ago),
        ))
        .await?;

        repo.save(DataPoint::new(
            DeviceStateValue::Temperature(Temperature::Bedroom, DegreeCelsius(19.0)),
            t!(22 minutes ago),
        ))
        .await?;

        Ok(())
    }
}
