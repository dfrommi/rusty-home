use chrono::{DateTime, Utc};

use support::unit::{DegreeCelsius, KiloWattHours, OpenedState, Percent, PowerState, Watt};

pub(super) mod db;

#[derive(Debug, Clone)]
pub struct DataPoint<V> {
    pub value: V,
    pub timestamp: DateTime<Utc>,
}

pub trait ChannelId {
    type ValueType;
}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct DbChannelId {
    pub channel_name: &'static str,
    pub item_name: &'static str,
}

impl ChannelId for Temperature {
    type ValueType = DegreeCelsius;
}

impl From<&Temperature> for DbChannelId {
    fn from(val: &Temperature) -> Self {
        DbChannelId {
            channel_name: "temperature",
            item_name: val.into(),
        }
    }
}

impl ChannelId for RelativeHumidity {
    type ValueType = Percent;
}

impl From<&RelativeHumidity> for DbChannelId {
    fn from(val: &RelativeHumidity) -> Self {
        DbChannelId {
            channel_name: "relative_humidity",
            item_name: val.into(),
        }
    }
}

impl ChannelId for Opened {
    type ValueType = OpenedState;
}

impl From<&Opened> for DbChannelId {
    fn from(val: &Opened) -> Self {
        DbChannelId {
            channel_name: "opened",
            item_name: val.into(),
        }
    }
}

impl ChannelId for Powered {
    type ValueType = PowerState;
}

impl From<&Powered> for DbChannelId {
    fn from(val: &Powered) -> Self {
        DbChannelId {
            channel_name: "powered",
            item_name: val.into(),
        }
    }
}

impl ChannelId for CurrentPowerUsage {
    type ValueType = Watt;
}

impl From<&CurrentPowerUsage> for DbChannelId {
    fn from(value: &CurrentPowerUsage) -> Self {
        DbChannelId {
            channel_name: "current_power_usage",
            item_name: value.into(),
        }
    }
}

impl ChannelId for TotalEnergyConsumption {
    type ValueType = KiloWattHours;
}

impl From<&TotalEnergyConsumption> for DbChannelId {
    fn from(value: &TotalEnergyConsumption) -> Self {
        DbChannelId {
            channel_name: "total_energy_consumption",
            item_name: value.into(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum ChannelValue {
    Temperature(Temperature, DegreeCelsius),
    RelativeHumidity(RelativeHumidity, Percent),
    Opened(Opened, OpenedState),
    Powered(Powered, PowerState),
    CurrentPowerUsage(CurrentPowerUsage, Watt),
    TotalEnergyConsumption(TotalEnergyConsumption, KiloWattHours),
}

impl From<&ChannelValue> for DbChannelId {
    fn from(value: &ChannelValue) -> Self {
        match value {
            ChannelValue::Temperature(id, _) => id.into(),
            ChannelValue::RelativeHumidity(id, _) => id.into(),
            ChannelValue::Opened(id, _) => id.into(),
            ChannelValue::Powered(id, _) => id.into(),
            ChannelValue::CurrentPowerUsage(id, _) => id.into(),
            ChannelValue::TotalEnergyConsumption(id, _) => id.into(),
        }
    }
}

impl From<&ChannelValue> for f64 {
    fn from(val: &ChannelValue) -> Self {
        match val {
            ChannelValue::Temperature(_, v) => v.into(),
            ChannelValue::RelativeHumidity(_, v) => v.into(),
            ChannelValue::Opened(_, v) => v.into(),
            ChannelValue::Powered(_, v) => v.into(),
            ChannelValue::CurrentPowerUsage(_, v) => v.into(),
            ChannelValue::TotalEnergyConsumption(_, v) => v.into(),
        }
    }
}

#[derive(Debug, Clone, Hash, Eq, PartialEq, strum::IntoStaticStr)]
#[strum(serialize_all = "snake_case")]
pub enum Temperature {
    Outside,
    LivingRoomDoor,
    RoomOfRequirementsDoor,
    BedroomDoor,
    BedroomOuterWall,
    KitchenOuterWall,
    BathroomShower,
}

#[derive(Debug, Clone, Hash, Eq, PartialEq, strum::IntoStaticStr)]
#[strum(serialize_all = "snake_case")]
pub enum RelativeHumidity {
    Outside,
    LivingRoomDoor,
    RoomOfRequirementsDoor,
    BedroomDoor,
    BedroomOuterWall,
    KitchenOuterWall,
    BathroomShower,
}

#[derive(Debug, Clone, Hash, Eq, PartialEq, strum::IntoStaticStr)]
#[strum(serialize_all = "snake_case")]
pub enum Opened {
    KitchenWindow,
    BedroomWindow,
    LivingRoomWindowLeft,
    LivingRoomWindowRight,
    LivingRoomWindowSide,
    LivingRoomBalconyDoor,
    RoomOfRequirementsWindowLeft,
    RoomOfRequirementsWindowRight,
    RoomOfRequirementsWindowSide,
}

#[derive(Debug, Clone, Hash, Eq, PartialEq, strum::IntoStaticStr)]
#[strum(serialize_all = "snake_case")]
pub enum Powered {
    Dehumidifier,
}

#[derive(Debug, Clone, Hash, Eq, PartialEq, strum::IntoStaticStr)]
#[strum(serialize_all = "snake_case")]
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
}

#[derive(Debug, Clone, Hash, Eq, PartialEq, strum::IntoStaticStr)]
#[strum(serialize_all = "snake_case")]
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
}
