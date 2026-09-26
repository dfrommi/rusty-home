mod availability;
mod cached;
mod state;

pub use availability::DeviceAvailabilityRepository;
pub use cached::CachedDeviceStateRepository;
pub use state::PostgresDeviceStateRepository;

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use std::collections::HashSet;

    use sqlx::PgPool;

    use crate::{
        core::{time::DateTimeRange, timeseries::DataPoint, unit::DegreeCelsius},
        device_state::{
            DeviceAvailabilityConfig, DeviceAvailabilityItem, DeviceAvailabilityOverride, DeviceStateId,
            DeviceStateValue, Temperature,
        },
        t,
    };

    use super::{CachedDeviceStateRepository, DeviceAvailabilityRepository, PostgresDeviceStateRepository};

    fn default_availability_config() -> DeviceAvailabilityConfig {
        DeviceAvailabilityConfig {
            default_offline_after: t!(1 hours),
            overrides: vec![],
        }
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn get_all_data_points_in_range_ts_asc(pool: PgPool) -> anyhow::Result<()> {
        let repo = PostgresDeviceStateRepository::new(pool);
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
    async fn get_latest_for_device(pool: PgPool) -> anyhow::Result<()> {
        let repo = PostgresDeviceStateRepository::new(pool);
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
    async fn get_latest_for_device_no_data(pool: PgPool) -> anyhow::Result<()> {
        let repo = PostgresDeviceStateRepository::new(pool);

        let dp = repo
            .get_latest_for_device(&DeviceStateId::Temperature(Temperature::LivingRoom))
            .await?;

        assert!(dp.is_none());

        Ok(())
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn cached_latest_state_updates_on_save_and_bypasses_cache_during_timeshift(
        pool: PgPool,
    ) -> anyhow::Result<()> {
        let repo = CachedDeviceStateRepository::new(PostgresDeviceStateRepository::new(pool));
        let id = DeviceStateId::Temperature(Temperature::LivingRoom);
        let first = DataPoint::new(
            DeviceStateValue::Temperature(Temperature::LivingRoom, DegreeCelsius(20.5)),
            t!(2 hours ago),
        );
        let latest = DataPoint::new(
            DeviceStateValue::Temperature(Temperature::LivingRoom, DegreeCelsius(21.0)),
            t!(1 hours ago),
        );

        assert!(repo.save(first.clone()).await?);
        assert!(repo.save(latest.clone()).await?);

        let cached = repo.get_latest_for_device(&id).await?.expect("latest value not found");
        assert_eq!(cached.value, latest.value);
        assert_eq!(cached.timestamp, latest.timestamp);

        let shifted_now = t!(90 minutes ago);
        let shifted = shifted_now
            .eval_timeshifted(async { repo.get_latest_for_device(&id).await })
            .await?
            .expect("shifted value not found");
        assert_eq!(shifted.value, first.value);
        assert_eq!(shifted.timestamp, first.timestamp);

        let current = repo.get_latest_for_device(&id).await?.expect("latest value not found");
        assert_eq!(current.value, latest.value);
        assert_eq!(current.timestamp, latest.timestamp);

        Ok(())
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn sync_item_availability_reconciles_existing_and_missing_items(pool: PgPool) -> anyhow::Result<()> {
        let repo = DeviceAvailabilityRepository::new(pool.clone(), default_availability_config());
        let now = t!(now).into_db();

        sqlx::query!(
            r#"INSERT INTO item_availability
                (source, item, last_seen, marked_offline, entry_updated, disabled)
                VALUES ($1, $2, $3, true, $3, true)"#,
            "Tasmota",
            "existing",
            now,
        )
        .execute(&pool)
        .await?;

        sqlx::query!(
            r#"INSERT INTO item_availability
                (source, item, last_seen, marked_offline, entry_updated, disabled)
                VALUES ($1, $2, $3, false, $3, false)"#,
            "Tasmota",
            "removed",
            now,
        )
        .execute(&pool)
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
        .fetch_all(&pool)
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
        let repo = DeviceAvailabilityRepository::new(
            pool.clone(),
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
        .execute(&pool)
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
    async fn get_all_data_points_ignores_unsupported_tag(pool: PgPool) -> anyhow::Result<()> {
        let repo = PostgresDeviceStateRepository::new(pool.clone());

        let tag_id = sqlx::query_scalar!(
            r#"INSERT INTO thing_value_tag (channel, name) VALUES ($1, $2) RETURNING id as "id!""#,
            "removed_state",
            "removed_device",
        )
        .fetch_one(&pool)
        .await?;

        sqlx::query!(
            r#"INSERT INTO thing_value (tag_id, value, timestamp) VALUES ($1, $2, $3)"#,
            tag_id,
            1.0,
            t!(now).into_db(),
        )
        .execute(&pool)
        .await?;

        let dps = repo
            .get_all_data_points_in_range_ts_asc(DateTimeRange::since(t!(1 hours ago)))
            .await?;

        assert!(dps.is_empty());

        Ok(())
    }

    async fn prepare_test_data(repo: &PostgresDeviceStateRepository) -> anyhow::Result<()> {
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
