use r#macro::{EnumVariants, Id};

#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq, Id, EnumVariants)]
pub enum TotalEnergyConsumption {
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
