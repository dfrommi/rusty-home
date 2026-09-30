use crate::core::time::DateTime;
use crate::core::timeseries::DataPoint;
use crate::core::unit::HeatingUnit;
use crate::device_state::adapter::{IncomingData, IncomingDataSource};
use crate::device_state::{DeviceAvailability, DeviceAvailabilityItem, DeviceStateValue, TotalRadiatorConsumption};
use infrastructure::EventListener;

use crate::frontends::energy_meter::{EnergyMeterTarget, EnergyReading, Radiator};

const AVAILABILITY_SOURCE: &str = "EnergyMeter";

pub struct EnergyMeterIncomingDataSource {
    rx: EventListener<DataPoint<EnergyReading>>,
}

impl EnergyMeterIncomingDataSource {
    pub fn new(rx: EventListener<DataPoint<EnergyReading>>) -> Self {
        Self { rx }
    }
}

impl IncomingDataSource for EnergyMeterIncomingDataSource {
    fn availability_items(&self) -> Vec<DeviceAvailabilityItem> {
        EnergyMeterTarget::variants()
            .iter()
            .map(|target| DeviceAvailabilityItem::new(AVAILABILITY_SOURCE, target.ext_id().variant_name().to_string()))
            .collect()
    }

    async fn recv_multi(&mut self) -> Option<Vec<IncomingData>> {
        let dp = self.rx.recv().await?;
        let availability = availability(&dp.value, dp.timestamp);

        Some(vec![IncomingData::StateValue(dp.map_value(|v| v.into())), availability])
    }
}

fn availability(reading: &EnergyReading, last_seen: DateTime) -> IncomingData {
    let target = reading.target();

    DeviceAvailability {
        item: DeviceAvailabilityItem::new(AVAILABILITY_SOURCE, target.ext_id().variant_name().to_string()),
        last_seen,
        marked_offline: false,
    }
    .into()
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
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use infrastructure::EventBus;

    #[tokio::test]
    async fn cumulative_reading_is_mapped_without_changing_its_value_or_timestamp() {
        let bus = EventBus::new(8);
        let mut ds = EnergyMeterIncomingDataSource::new(bus.subscribe());
        let timestamp = DateTime::from_static_iso("2025-01-01T12:00:00Z");

        bus.emitter()
            .send(DataPoint::new(EnergyReading::Heating(Radiator::Bedroom, 312.5), timestamp));

        let updates = ds.recv_multi().await.expect("energy meter source closed");

        assert_eq!(updates.len(), 2);
        match &updates[0] {
            IncomingData::StateValue(dp) => {
                assert_eq!(
                    dp.value,
                    DeviceStateValue::TotalRadiatorConsumption(TotalRadiatorConsumption::Bedroom, HeatingUnit(312.5))
                );
                assert_eq!(dp.timestamp, timestamp);
            }
            IncomingData::ItemAvailability(_) => panic!("expected state value"),
        }
        match &updates[1] {
            IncomingData::ItemAvailability(availability) => {
                assert_eq!(availability.item.source, AVAILABILITY_SOURCE);
                assert_eq!(availability.item.item, "heating::bedroom");
                assert_eq!(availability.last_seen, timestamp);
                assert!(!availability.marked_offline);
            }
            IncomingData::StateValue(_) => panic!("expected availability"),
        }
    }
}
