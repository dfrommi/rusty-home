mod availability;
mod availability_sqlite;

mod state;
mod state_sqlite;

pub use availability::DeviceAvailabilityRepository;
pub use availability_sqlite::SQLiteDeviceAvailabilityRepository;

pub use state::{PostgresDeviceStateRepository, SaveOutcome};
pub use state_sqlite::SQLiteDeviceStateRepository;

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use std::collections::HashSet;

    use sqlx::PgPool;

    use crate::{
        device_state::{DeviceAvailabilityConfig, DeviceAvailabilityItem, DeviceAvailabilityOverride},
        t,
    };

    use super::DeviceAvailabilityRepository;

    fn default_availability_config() -> DeviceAvailabilityConfig {
        DeviceAvailabilityConfig {
            default_offline_after: t!(1 hours),
            overrides: vec![],
        }
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
}
