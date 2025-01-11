use sqlx::postgres::types::PgInterval;
use support::t;

use crate::{
    core::{domain::ItemAvailabilityStorage, ItemAvailability},
    Database,
};

impl ItemAvailabilityStorage for Database {
    async fn add_item_availability(&self, item: ItemAvailability) -> anyhow::Result<()> {
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
        .execute(&self.db_pool)
        .await?;

        Ok(())
    }
}
