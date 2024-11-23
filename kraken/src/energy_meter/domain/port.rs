use support::time::DateTime;

use super::EnergyReading;

#[trait_variant::make(Send)] //for axum
pub trait AddEnergyReadingUseCase {
    async fn add_energy_reading(&self, reading: EnergyReading) -> anyhow::Result<()>;
}

#[trait_variant::make(Send)] //for axum
pub trait EnergyReadingRepository {
    async fn add_energy_reading(
        &self,
        reading: EnergyReading,
        timestamp: DateTime,
    ) -> anyhow::Result<()>;
}
