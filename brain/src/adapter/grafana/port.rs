use support::time::Duration;

pub struct OfflineItem {
    pub source: String,
    pub item: String,
    pub duration: Duration,
}

pub trait ItemAvailabilitySupportStorage {
    async fn get_offline_items(&self) -> anyhow::Result<Vec<OfflineItem>>;
}
