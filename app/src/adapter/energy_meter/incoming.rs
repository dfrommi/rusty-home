use crate::adapter::incoming::{IncomingData, IncomingDataSource};
use crate::core::unit::{HeatingUnit, KiloCubicMeter};
use crate::device_state::{DeviceStateValue, TotalRadiatorConsumption, TotalWaterConsumption};
use tokio::sync::broadcast::Receiver;

use crate::{core::app_event::EnergyReadingAddedEvent, core::persistence::Database};

use super::{EnergyReading, Faucet, Radiator};

pub struct EnergyMeterIncomingDataSource {
    db: Database,
    rx: Receiver<EnergyReadingAddedEvent>,
    initial_load: Option<Vec<EnergyReadingAddedEvent>>,
}

impl EnergyMeterIncomingDataSource {
    pub fn new(db: Database, rx: Receiver<EnergyReadingAddedEvent>) -> Self {
        Self {
            db,
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
            self.initial_load = match self.db.get_latest_total_readings_ids().await {
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
                Ok(msg) => Some(msg),
                Err(e) => {
                    tracing::error!("Error receiving energy reading: {}", e);
                    None
                }
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
        let dp = self.db.get_total_reading_by_id(msg.id).await?;
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
