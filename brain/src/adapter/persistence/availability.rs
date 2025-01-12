use sqlx::postgres::types::PgInterval;
use support::{t, time::Duration};

use crate::adapter::grafana::{ItemAvailabilitySupportStorage, OfflineItem};

impl ItemAvailabilitySupportStorage for super::Database {
    async fn get_offline_items(&self) -> anyhow::Result<Vec<OfflineItem>> {
        let recs = sqlx::query!(
            r#"SELECT source, item, last_seen, marked_offline, considered_offline_after, entry_updated
                FROM item_availability"#
        )
        .fetch_all(&self.pool)
        .await?;

        let mut offline_items = vec![];

        for rec in recs.iter() {
            let considered_offline_after =
                convert_pginterval_to_duration(&rec.considered_offline_after);
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
