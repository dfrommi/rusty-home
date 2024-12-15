use api::{
    state::{ChannelValue, TotalRadiatorConsumption, TotalWaterConsumption},
    EnergyReadingInsertEvent,
};
use support::{
    t,
    unit::{HeatingUnit, KiloCubicMeter},
    DataPoint,
};
use tokio::sync::broadcast::Receiver;

use crate::core::StateCollector;

use super::{AddEnergyReadingUseCase, EnergyReading, EnergyReadingRepository, Faucet, Radiator};

#[derive(Clone)]
pub struct EnergyMeterService<R> {
    repo: R,
}

pub struct EnergyMeterStateCollector<R> {
    repo: R,
    rx: Receiver<EnergyReadingInsertEvent>,
}

impl<R> EnergyMeterService<R> {
    pub fn new(repo: R) -> Self {
        Self { repo }
    }
}

impl<R> EnergyMeterStateCollector<R> {
    pub fn new(repo: R, rx: Receiver<EnergyReadingInsertEvent>) -> Self {
        Self { repo, rx }
    }
}

//PORT IN

impl<R> AddEnergyReadingUseCase for EnergyMeterService<R>
where
    R: EnergyReadingRepository + Send + Clone + Sync,
{
    async fn add_energy_reading(&self, reading: EnergyReading) -> anyhow::Result<()> {
        self.repo.add_yearly_energy_reading(reading, t!(now)).await
    }
}

impl<R> StateCollector for EnergyMeterStateCollector<R>
where
    R: EnergyReadingRepository,
{
    async fn get_current_state(&self) -> anyhow::Result<Vec<DataPoint<ChannelValue>>> {
        let readings = self.repo.get_latest_total_readings().await?;

        let dps: Vec<DataPoint<ChannelValue>> = readings
            .into_iter()
            .map(|dp| dp.map_value(|v| v.into()))
            .collect();

        Ok(dps)
    }

    async fn recv(&mut self) -> anyhow::Result<DataPoint<ChannelValue>> {
        loop {
            match self.rx.recv().await {
                Ok(EnergyReadingInsertEvent { id }) => {
                    tracing::info!("Received energy reading with id {}", id);
                    match self.repo.get_total_reading_by_id(id).await {
                        Ok(dp) => return Ok(dp.map_value(|v| v.into())),
                        Err(e) => {
                            tracing::error!(
                                "Error getting energy reading with id {} in event handler: {}",
                                id,
                                e
                            );
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("Error receiving energy reading: {}", e);
                    tokio::task::yield_now().await
                }
            }
        }
    }
}

impl From<&EnergyReading> for ChannelValue {
    fn from(val: &EnergyReading) -> Self {
        match val {
            EnergyReading::Heating(item, value) => ChannelValue::TotalRadiatorConsumption(
                match item {
                    Radiator::LivingRoomBig => TotalRadiatorConsumption::LivingRoomBig,
                    Radiator::LivingRoomSmall => TotalRadiatorConsumption::LivingRoomSmall,
                    Radiator::Bedroom => TotalRadiatorConsumption::Bedroom,
                    Radiator::Kitchen => TotalRadiatorConsumption::Kitchen,
                    Radiator::RoomOfRequirements => TotalRadiatorConsumption::RoomOfRequirements,
                    Radiator::Bathroom => TotalRadiatorConsumption::Bathroom,
                },
                HeatingUnit(*value),
            ),
            EnergyReading::ColdWater(item, value) => ChannelValue::TotalWaterConsumption(
                match item {
                    Faucet::Kitchen => TotalWaterConsumption::KitchenCold,
                    Faucet::Bathroom => TotalWaterConsumption::BathroomCold,
                },
                KiloCubicMeter(*value),
            ),
            EnergyReading::HotWater(item, value) => ChannelValue::TotalWaterConsumption(
                match item {
                    Faucet::Kitchen => TotalWaterConsumption::KitchenWarm,
                    Faucet::Bathroom => TotalWaterConsumption::BathroomWarm,
                },
                KiloCubicMeter(*value),
            ),
        }
    }
}
