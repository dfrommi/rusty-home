use crate::core::time::Duration;
use crate::t;
use sqlx::postgres::types::PgInterval;

use crate::core::ItemAvailability;

pub struct OfflineItem {
    pub source: String,
    pub item: String,
    pub duration: Duration,
}

// Item Availability & Health Monitoring
// Methods for tracking and monitoring the availability status of items/devices
impl super::Database {
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

    pub async fn add_item_availability(&self, item: ItemAvailability) -> anyhow::Result<()> {
        sqlx::query!(
            r#"INSERT INTO item_availability (source, item, last_seen, marked_offline, considered_offline_after, entry_updated)
                VALUES ($1, $2, $3, $4, $5, $6)
                ON CONFLICT (source, item) DO UPDATE SET last_seen = $3, marked_offline = $4, entry_updated = $6"#,
            item.source,
            item.item,
            item.last_seen.into_db(),
            item.marked_offline,
            //TODO should just work via chrono::Duration, but doesn't
            PgInterval::try_from(t!(1 hours).into_db()).unwrap(),
            t!(now).into_db(),
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}

fn convert_pginterval_to_duration(pg_interval: &PgInterval) -> Duration {
    let days_from_months = pg_interval.months * 30; // Rough estimation
    let total_days = days_from_months + pg_interval.days;

    let total_milliseconds = pg_interval.microseconds / 1_000;

    Duration::days(total_days as i64) + Duration::millis(total_milliseconds)
}
