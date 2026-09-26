use std::collections::HashSet;

use sqlx::PgPool;

use crate::{
    core::time::DateTime,
    device_state::{DeviceAvailabilityConfig, DeviceAvailabilityItem, DeviceAvailabilityStatus},
    t,
};

pub struct DeviceAvailabilityRepository {
    pool: PgPool,
    availability_config: DeviceAvailabilityConfig,
}

impl DeviceAvailabilityRepository {
    pub fn new(pool: PgPool, availability_config: DeviceAvailabilityConfig) -> Self {
        Self {
            pool,
            availability_config,
        }
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
}
