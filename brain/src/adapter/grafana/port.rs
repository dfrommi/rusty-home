pub struct OfflineItem {
    pub source: String,
    pub item: String,
}

pub trait ItemAvailabilitySupportStorage {
    async fn get_offline_items(&self) -> anyhow::Result<Vec<OfflineItem>>;
}
