mod persistence;

use crate::core::unit::{HeatingUnit, KiloCubicMeter};
use crate::device_state::adapter::energy_meter::persistence::EnergyReadingRepository;
use crate::device_state::adapter::{IncomingData, IncomingDataSource};
use crate::device_state::{DeviceStateValue, TotalRadiatorConsumption, TotalWaterConsumption};
use crate::t;
use tokio::sync::mpsc;

use crate::adapter::energy_meter::{EnergyReading, Faucet, Radiator};

#[derive(Debug, Clone)]
pub struct EnergyReadingAddedEvent {
    pub id: i64,
}

pub struct EnergyMeterIncomingDataSource {
    repo: EnergyReadingRepository,
    rx: mpsc::Receiver<EnergyReading>,
    initial_load: Option<Vec<EnergyReadingAddedEvent>>,
}

impl EnergyMeterIncomingDataSource {
    pub fn new(pool: sqlx::PgPool, rx: mpsc::Receiver<EnergyReading>) -> Self {
        let repo = EnergyReadingRepository::new(pool);
        Self {
            repo,
            rx,
            initial_load: None,
        }
    }
}

impl IncomingDataSource<EnergyReadingAddedEvent, ()> for EnergyMeterIncomingDataSource {
    fn ds_name(&self) -> &str {
        "EnergyMeter"
    }

    async fn recv(&mut self) -> Option<EnergyReadingAddedEvent> {
        if self.initial_load.is_none() {
            self.initial_load = match self.repo.get_latest_total_readings_ids().await {
                Ok(v) => Some(v.iter().map(|id| EnergyReadingAddedEvent { id: *id }).collect()),
                Err(e) => {
                    tracing::error!("Error loading initial state for Energy Reading: {:?}", e);
                    Some(vec![])
                }
            };
        }

        match &mut self.initial_load {
            Some(data) if !data.is_empty() => data.pop(),
            _ => match self.rx.recv().await {
                Some(msg) => match self.repo.add_yearly_energy_reading(msg, t!(now)).await {
                    Ok(id) => Some(EnergyReadingAddedEvent { id }),
                    Err(e) => {
                        tracing::error!("Error saving Energy Reading: {:?}", e);
                        return None;
                    }
                },
                None => None,
            },
        }
    }

    fn device_id(&self, msg: &EnergyReadingAddedEvent) -> Option<String> {
        Some(msg.id.to_string())
    }

    fn get_channels(&self, _: &str) -> &[()] {
        &[()]
    }

    async fn to_incoming_data(
        &self,
        _: &str,
        _: &(),
        msg: &EnergyReadingAddedEvent,
    ) -> anyhow::Result<Vec<IncomingData>> {
        let dp = self.repo.get_total_reading_by_id(msg.id).await?;
        Ok(vec![IncomingData::StateValue(dp.map_value(|v| v.into()))])
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
