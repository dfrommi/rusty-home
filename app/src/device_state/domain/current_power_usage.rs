use r#macro::{EnumVariants, Id};

use crate::core::unit::Watt;

#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq, Id, EnumVariants)]
pub enum CurrentPowerUsage {
    Fridge,
    Dehumidifier,
    AppleTv,
    Tv,
    AirPurifier,
    CouchLight,
    Dishwasher,
    Kettle,
    WashingMachine,
    Nuc,
    DslModem,
    InternetGateway,
    NetworkSwitch,
    KitchenMultiPlug,
    CouchPlug,
    RoomOfRequirementsDesk,
    RoomOfRequirementsMonitor,
    InfraredHeater,
}

impl CurrentPowerUsage {
    pub fn minimum_storage_delta(&self, _incoming_value: Watt) -> Option<Watt> {
        None
    }
}
