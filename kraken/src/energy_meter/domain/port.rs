use support::{time::DateTime, DataPoint};

use super::EnergyReading;

#[trait_variant::make(Send)] //for axum
pub trait AddEnergyReadingUseCase {
    async fn add_energy_reading(&self, reading: EnergyReading) -> anyhow::Result<()>;
}

#[trait_variant::make(Send)] //for axum
pub trait EnergyReadingRepository {
    async fn add_yearly_energy_reading(
        &self,
        reading: EnergyReading,
        timestamp: DateTime,
    ) -> anyhow::Result<()>;

    async fn get_latest_total_readings(&self) -> anyhow::Result<Vec<DataPoint<EnergyReading>>>;

    async fn get_total_reading_by_id(&self, id: i64) -> anyhow::Result<DataPoint<EnergyReading>>;
}
