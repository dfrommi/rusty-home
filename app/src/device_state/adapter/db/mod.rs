use anyhow::{Context as _, Result};
use moka::future::Cache;
use sqlx::PgPool;
use std::collections::HashSet;

use crate::{
    core::{
        id::ExternalId,
        time::{DateTime, DateTimeRange},
        timeseries::DataPoint,
    },
    device_state::{
        DeviceAvailabilityConfig, DeviceAvailabilityItem, DeviceAvailabilityStatus, DeviceStateId, DeviceStateValue,
    },
    t,
};

#[derive(Debug, Clone)]
pub struct DeviceStateRepository {
    pool: PgPool,
    availability_config: DeviceAvailabilityConfig,
    tag_id_cache: Cache<DeviceStateId, i64>,
}

impl DeviceStateRepository {
    pub fn new(pool: PgPool, availability_config: DeviceAvailabilityConfig) -> Self {
        Self {
            pool,
            availability_config,
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

    pub async fn update_device_availability(
        &self,
        device_id: &str,
        source: &str,
        last_seen: &DateTime,
        offline: bool,
    ) -> anyhow::Result<()> {
        sqlx::query!(
            r#"INSERT INTO item_availability (source, item, last_seen, marked_offline, entry_updated, disabled)
                VALUES ($1, $2, $3, $4, $5, false)
                ON CONFLICT (source, item) DO UPDATE SET last_seen = $3, marked_offline = $4, entry_updated = $5, disabled = false"#,
            source,
            device_id,
            last_seen.into_db(),
            offline,
            t!(now).into_db(),
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn sync_item_availability(&self, items: &HashSet<DeviceAvailabilityItem>) -> anyhow::Result<()> {
        self.availability_config.warn_unknown_items(items);

        let (sources, item_names): (Vec<String>, Vec<String>) = items
            .iter()
            .map(|item| (item.source.clone(), item.item.clone()))
            .unzip();
        let now = t!(now).into_db();
        let mut tx = self.pool.begin().await?;

        sqlx::query(
            r#"UPDATE item_availability AS existing
               SET disabled = NOT EXISTS (
                   SELECT 1
                   FROM UNNEST($1::text[], $2::text[]) AS configured(source, item)
                   WHERE configured.source = existing.source
                     AND configured.item = existing.item
               )"#,
        )
        .bind(&sources)
        .bind(&item_names)
        .execute(&mut *tx)
        .await?;

        sqlx::query(
            r#"INSERT INTO item_availability (
                   source, item, last_seen, marked_offline,
                   entry_updated, disabled
               )
               SELECT configured.source, configured.item, $3, true,
                      $3, false
               FROM UNNEST($1::text[], $2::text[]) AS configured(source, item)
               ON CONFLICT (source, item) DO UPDATE SET disabled = false"#,
        )
        .bind(&sources)
        .bind(&item_names)
        .bind(now)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(())
    }

    pub async fn get_item_availabilities(&self) -> anyhow::Result<Vec<DeviceAvailabilityStatus>> {
        let recs = sqlx::query!(
            r#"SELECT source, item, last_seen, marked_offline, entry_updated, disabled
                FROM item_availability"#
        )
        .fetch_all(&self.pool)
        .await?;

        let now = t!(now);

        Ok(recs
            .into_iter()
            .map(|rec| {
                let offline_after = if rec.disabled {
                    &self.availability_config.default_offline_after
                } else {
                    self.availability_config.offline_after(&rec.source, &rec.item)
                };
                let last_seen_ago = std::cmp::max(
                    now.elapsed_since(rec.last_seen.into()),
                    now.elapsed_since(rec.entry_updated.into()),
                );
                let is_offline = rec.marked_offline || last_seen_ago > offline_after.clone();

                DeviceAvailabilityStatus {
                    source: rec.source,
                    item: rec.item,
                    last_seen_ago,
                    is_offline,
                    disabled: rec.disabled,
                }
            })
            .collect())
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

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use std::collections::HashSet;

    use crate::{
        core::unit::DegreeCelsius,
        device_state::{DeviceAvailabilityOverride, Temperature},
    };

    use super::*;

    fn default_availability_config() -> DeviceAvailabilityConfig {
        DeviceAvailabilityConfig {
            default_offline_after: t!(1 hours),
            overrides: vec![],
        }
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn test_get_all_data_points_in_range_ts_asc(pool: PgPool) -> anyhow::Result<()> {
        let repo = DeviceStateRepository::new(pool, default_availability_config());
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
        let repo = DeviceStateRepository::new(pool, default_availability_config());
        prepare_test_data(&repo).await?;

        let dp = repo
            .get_latest_for_device(&DeviceStateId::Temperature(Temperature::LivingRoom))
            .await?
            .expect("expected a data point for existing device");

        assert_eq!(
            dp.value,
            DeviceStateValue::Temperature(Temperature::LivingRoom, DegreeCelsius(22.0))
        );

        Ok(())
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn test_get_latest_for_device_no_data(pool: PgPool) -> anyhow::Result<()> {
        let repo = DeviceStateRepository::new(pool, default_availability_config());
        // No data inserted — device has no rows in thing_value

        let dp = repo
            .get_latest_for_device(&DeviceStateId::Temperature(Temperature::LivingRoom))
            .await?;

        assert!(dp.is_none());

        Ok(())
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn sync_item_availability_reconciles_existing_and_missing_items(pool: PgPool) -> anyhow::Result<()> {
        let repo = DeviceStateRepository::new(pool, default_availability_config());
        let now = t!(now).into_db();

        sqlx::query!(
            r#"INSERT INTO item_availability
                (source, item, last_seen, marked_offline, entry_updated, disabled)
                VALUES ($1, $2, $3, true, $3, true)"#,
            "Tasmota",
            "existing",
            now,
        )
        .execute(&repo.pool)
        .await?;

        sqlx::query!(
            r#"INSERT INTO item_availability
                (source, item, last_seen, marked_offline, entry_updated, disabled)
                VALUES ($1, $2, $3, false, $3, false)"#,
            "Tasmota",
            "removed",
            now,
        )
        .execute(&repo.pool)
        .await?;

        let items = HashSet::from([
            DeviceAvailabilityItem::new("Tasmota", "existing"),
            DeviceAvailabilityItem::new("Tasmota", "missing"),
        ]);
        repo.sync_item_availability(&items).await?;

        let rows = sqlx::query!(
            r#"SELECT item, marked_offline, disabled
               FROM item_availability
               ORDER BY item"#
        )
        .fetch_all(&repo.pool)
        .await?;

        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].item, "existing");
        assert!(rows[0].marked_offline);
        assert!(!rows[0].disabled);
        assert_eq!(rows[1].item, "missing");
        assert!(rows[1].marked_offline);
        assert!(!rows[1].disabled);
        assert_eq!(rows[2].item, "removed");
        assert!(!rows[2].marked_offline);
        assert!(rows[2].disabled);

        Ok(())
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn get_item_availabilities_uses_configured_duration(pool: PgPool) -> anyhow::Result<()> {
        let repo = DeviceStateRepository::new(
            pool,
            DeviceAvailabilityConfig {
                default_offline_after: t!(1 hours),
                overrides: vec![DeviceAvailabilityOverride {
                    source: "HA".to_string(),
                    item: "sensor.home_temperature".to_string(),
                    offline_after: t!(3 hours),
                }],
            },
        );
        let last_seen = t!(2 hours ago).into_db();

        sqlx::query(
            r#"INSERT INTO item_availability
                (source, item, last_seen, marked_offline, entry_updated, disabled)
                VALUES
                    ('HA', 'sensor.home_temperature', $1, false, $1, false),
                    ('HA', 'sensor.home_relative_humidity', $1, false, $1, false)"#,
        )
        .bind(last_seen)
        .execute(&repo.pool)
        .await?;

        let statuses = repo.get_item_availabilities().await?;
        let override_status = statuses
            .iter()
            .find(|status| status.item == "sensor.home_temperature")
            .expect("configured override status not found");
        let default_status = statuses
            .iter()
            .find(|status| status.item == "sensor.home_relative_humidity")
            .expect("default status not found");

        assert!(!override_status.is_offline);
        assert!(default_status.is_offline);

        Ok(())
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn test_get_all_data_points_ignores_unsupported_tag(pool: PgPool) -> anyhow::Result<()> {
        let repo = DeviceStateRepository::new(pool, default_availability_config());

        let tag_id = sqlx::query_scalar!(
            r#"INSERT INTO thing_value_tag (channel, name) VALUES ($1, $2) RETURNING id as "id!""#,
            "removed_state",
            "removed_device",
        )
        .fetch_one(&repo.pool)
        .await?;

        sqlx::query!(
            r#"INSERT INTO thing_value (tag_id, value, timestamp) VALUES ($1, $2, $3)"#,
            tag_id,
            1.0,
            t!(now).into_db(),
        )
        .execute(&repo.pool)
        .await?;

        let dps = repo
            .get_all_data_points_in_range_ts_asc(DateTimeRange::since(t!(1 hours ago)))
            .await?;

        assert!(dps.is_empty());

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
