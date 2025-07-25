use crate::core::time::DateTime;
use crate::core::unit::KiloWattHours;
use r#macro::{EnumVariants, Id};

use crate::core::timeseries::{
    DataFrame,
    interpolate::{Estimatable, algo},
};

#[derive(Debug, Clone, Hash, Eq, PartialEq, Id, EnumVariants)]
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
    InfraredHeater,
}

impl Estimatable for TotalEnergyConsumption {
    fn interpolate(&self, at: DateTime, df: &DataFrame<KiloWattHours>) -> Option<KiloWattHours> {
        algo::linear(at, df)
    }
}
