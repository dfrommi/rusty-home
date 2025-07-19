use r#macro::{EnumVariants, Id};

use crate::core::{
    time::DateTime,
    timeseries::{
        DataFrame,
        interpolate::{self, Estimatable},
    },
};

use super::Watt;

#[derive(Debug, Clone, Hash, Eq, PartialEq, Id, EnumVariants)]
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
    InfraredHeater,
}

impl Estimatable for CurrentPowerUsage {
    fn interpolate(&self, at: DateTime, df: &DataFrame<Watt>) -> Option<Watt> {
        interpolate::algo::last_seen(at, df)
    }
}
