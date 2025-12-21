use crate::core::timeseries::DataPoint;
use crate::home_state::calc::{DerivedStateProvider, StateCalculationContext};
use anyhow::Result;
use r#macro::{EnumVariants, Id};

#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq, EnumVariants, Id)]
pub enum OpenedArea {
    LivingRoomWindowOrDoor,
    BedroomWindow,
    KitchenWindow,
    RoomOfRequirementsWindow,

    //TODO remove
    KitchenRadiatorThermostat,
    BedroomRadiatorThermostat,
    LivingRoomRadiatorThermostatSmall,
    LivingRoomRadiatorThermostatBig,
    RoomOfRequirementsThermostat,
    BathroomThermostat,
}

pub struct OpenedAreaStateProvider;

impl DerivedStateProvider<OpenedArea, bool> for OpenedAreaStateProvider {
    fn calculate_current(&self, id: OpenedArea, ctx: &StateCalculationContext) -> Option<bool> {
        use crate::device_state::Opened as DeviceOpened;

        let opened_items = match id {
            OpenedArea::LivingRoomWindowOrDoor => vec![
                DeviceOpened::LivingRoomWindowLeft,
                DeviceOpened::LivingRoomWindowRight,
                DeviceOpened::LivingRoomWindowSide,
                DeviceOpened::LivingRoomBalconyDoor,
            ],
            OpenedArea::BedroomWindow => vec![DeviceOpened::BedroomWindow],
            OpenedArea::KitchenWindow => vec![DeviceOpened::KitchenWindow],
            OpenedArea::RoomOfRequirementsWindow => vec![
                DeviceOpened::RoomOfRequirementsWindowLeft,
                DeviceOpened::RoomOfRequirementsWindowRight,
                DeviceOpened::RoomOfRequirementsWindowSide,
            ],
            OpenedArea::KitchenRadiatorThermostat => vec![DeviceOpened::KitchenRadiatorThermostat],
            OpenedArea::BedroomRadiatorThermostat => vec![DeviceOpened::BedroomRadiatorThermostat],
            OpenedArea::LivingRoomRadiatorThermostatSmall => {
                vec![DeviceOpened::LivingRoomRadiatorThermostatSmall]
            }
            OpenedArea::LivingRoomRadiatorThermostatBig => {
                vec![DeviceOpened::LivingRoomRadiatorThermostatBig]
            }
            OpenedArea::RoomOfRequirementsThermostat => {
                vec![DeviceOpened::RoomOfRequirementsThermostat]
            }
            OpenedArea::BathroomThermostat => {
                vec![DeviceOpened::BathroomThermostat]
            }
        };

        let opened_dps: Vec<_> = opened_items
            .iter()
            .filter_map(|o| ctx.device_state(o.clone()))
            .collect();

        if opened_dps.is_empty() {
            return None;
        }

        Some(any_of(opened_dps))
    }
}

fn any_of(opened_dps: Vec<DataPoint<bool>>) -> bool {
    opened_dps.iter().any(|v| v.value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::t;

    #[tokio::test]
    async fn test_any_of_some_opened() {
        let res = any_of(vec![
            DataPoint::new(false, t!(5 minutes ago)),
            DataPoint::new(true, t!(3 minutes ago)),
            DataPoint::new(false, t!(1 minutes ago)),
        ]);

        assert!(res);
    }

    #[tokio::test]
    async fn test_any_of_all_closed() {
        let res = any_of(vec![
            DataPoint::new(false, t!(5 minutes ago)),
            DataPoint::new(false, t!(3 minutes ago)),
            DataPoint::new(false, t!(1 minutes ago)),
        ]);

        assert!(!res);
    }
}
