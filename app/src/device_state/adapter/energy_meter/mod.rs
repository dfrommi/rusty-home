mod persistence;

use crate::core::unit::{HeatingUnit, KiloCubicMeter};
use crate::device_state::adapter::energy_meter::persistence::EnergyReadingRepository;
use crate::device_state::adapter::{IncomingData, IncomingDataSource};
use crate::device_state::{DeviceStateValue, TotalRadiatorConsumption, TotalWaterConsumption};
use crate::t;
use infrastructure::EventListener;

use crate::frontends::energy_meter::{EnergyReading, Faucet, Radiator};

pub struct EnergyMeterIncomingDataSource {
    repo: EnergyReadingRepository,
    rx: EventListener<EnergyReading>,
    initial_load: Option<Vec<i64>>,
}

impl EnergyMeterIncomingDataSource {
    pub fn new(pool: sqlx::PgPool, rx: EventListener<EnergyReading>) -> Self {
        let repo = EnergyReadingRepository::new(pool);
        Self {
            repo,
            rx,
            initial_load: None,
        }
    }

    async fn incoming_data_for_reading_id(&self, id: i64) -> anyhow::Result<Vec<IncomingData>> {
        let dp = self.repo.get_total_reading_by_id(id).await?;
        Ok(vec![IncomingData::StateValue(dp.map_value(|v| v.into()))])
    }
}

impl IncomingDataSource for EnergyMeterIncomingDataSource {
    async fn recv_multi(&mut self) -> Option<Vec<IncomingData>> {
        loop {
            if self.initial_load.is_none() {
                self.initial_load = match self.repo.get_latest_total_readings_ids().await {
                    Ok(ids) => Some(ids),
                    Err(e) => {
                        tracing::error!("Error loading initial state for Energy Reading: {:?}", e);
                        Some(vec![])
                    }
                };
            }

            if let Some(id) = self.initial_load.as_mut().and_then(|data| data.pop()) {
                match self.incoming_data_for_reading_id(id).await {
                    Ok(data) => return Some(data),
                    Err(e) => {
                        tracing::error!("Error loading Energy Reading with id {}: {:?}", id, e);
                        continue;
                    }
                }
            }

            let msg = self.rx.recv().await?;
            match self.repo.add_yearly_energy_reading(msg, t!(now)).await {
                Ok(id) => match self.incoming_data_for_reading_id(id).await {
                    Ok(data) => return Some(data),
                    Err(e) => tracing::error!("Error loading Energy Reading with id {}: {:?}", id, e),
                },
                Err(e) => tracing::error!("Error saving Energy Reading: {:?}", e),
            }
        }
    }
}

impl From<&EnergyReading> for DeviceStateValue {
    fn from(val: &EnergyReading) -> Self {
        match val {
            EnergyReading::Heating(item, value) => DeviceStateValue::TotalRadiatorConsumption(
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
            EnergyReading::ColdWater(item, value) => DeviceStateValue::TotalWaterConsumption(
                match item {
                    Faucet::Kitchen => TotalWaterConsumption::KitchenCold,
                    Faucet::Bathroom => TotalWaterConsumption::BathroomCold,
                },
                KiloCubicMeter(*value),
            ),
            EnergyReading::HotWater(item, value) => DeviceStateValue::TotalWaterConsumption(
                match item {
                    Faucet::Kitchen => TotalWaterConsumption::KitchenWarm,
                    Faucet::Bathroom => TotalWaterConsumption::BathroomWarm,
                },
                KiloCubicMeter(*value),
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use infrastructure::EventBus;

    use super::*;

    #[sqlx::test(migrations = "../migrations")]
    async fn live_reading_is_saved_then_emitted_as_total(pool: sqlx::PgPool) -> anyhow::Result<()> {
        let bus = EventBus::new(8);
        let mut ds = EnergyMeterIncomingDataSource::new(pool, bus.subscribe());

        bus.emitter().send(EnergyReading::Heating(Radiator::Bedroom, 12.5));

        let updates = ds.recv_multi().await.expect("energy meter source closed");

        assert_eq!(updates.len(), 1);
        match &updates[0] {
            IncomingData::StateValue(dp) => assert_eq!(
                dp.value,
                DeviceStateValue::TotalRadiatorConsumption(TotalRadiatorConsumption::Bedroom, HeatingUnit(12.5))
            ),
            IncomingData::ItemAvailability(_) => panic!("expected state value"),
        }

        Ok(())
    }
}
