use db::DbValue;
use r#macro::{DbMapped, EnumVariants, Id, StateChannel};
use support::unit::*;

pub mod db;

#[derive(Debug, Clone, StateChannel, DbMapped)]
pub enum ChannelValue {
    Temperature(Temperature, DegreeCelsius),
    RelativeHumidity(RelativeHumidity, Percent),
    Opened(Opened, bool),
    Powered(Powered, bool),
    CurrentPowerUsage(CurrentPowerUsage, Watt),
    TotalEnergyConsumption(TotalEnergyConsumption, KiloWattHours),
    SetPoint(SetPoint, DegreeCelsius),
    HeatingDemand(HeatingDemand, Percent),
    ExternalAutoControl(ExternalAutoControl, bool),
    Presence(Presence, bool),
    TotalRadiatorConsumption(TotalRadiatorConsumption, HeatingUnit),
    TotalWaterConsumption(TotalWaterConsumption, KiloCubicMeter),
    FanActivity(FanActivity, unit::FanAirflow),
}

impl ChannelValue {
    pub fn value_to_string(&self) -> String {
        match self {
            ChannelValue::Temperature(_, value) => value.to_string(),
            ChannelValue::RelativeHumidity(_, value) => value.to_string(),
            ChannelValue::Opened(_, value) => value.to_string(),
            ChannelValue::Powered(_, value) => value.to_string(),
            ChannelValue::CurrentPowerUsage(_, value) => value.to_string(),
            ChannelValue::TotalEnergyConsumption(_, value) => value.to_string(),
            ChannelValue::SetPoint(_, value) => value.to_string(),
            ChannelValue::HeatingDemand(_, value) => value.to_string(),
            ChannelValue::ExternalAutoControl(_, value) => value.to_string(),
            ChannelValue::Presence(_, value) => value.to_string(),
            ChannelValue::TotalRadiatorConsumption(_, value) => value.to_string(),
            ChannelValue::TotalWaterConsumption(_, value) => value.to_string(),
            ChannelValue::FanActivity(_, value) => value.to_string(),
        }
    }
}

//TODO macro
impl From<(Channel, DbValue)> for ChannelValue {
    fn from(val: (Channel, DbValue)) -> Self {
        let (channel, value) = val;
        match channel {
            Channel::Temperature(item) => ChannelValue::Temperature(item, value.into()),
            Channel::RelativeHumidity(item) => ChannelValue::RelativeHumidity(item, value.into()),
            Channel::Opened(item) => ChannelValue::Opened(item, value.into()),
            Channel::Powered(item) => ChannelValue::Powered(item, value.into()),
            Channel::CurrentPowerUsage(item) => ChannelValue::CurrentPowerUsage(item, value.into()),
            Channel::TotalEnergyConsumption(item) => {
                ChannelValue::TotalEnergyConsumption(item, value.into())
            }
            Channel::SetPoint(item) => ChannelValue::SetPoint(item, value.into()),
            Channel::HeatingDemand(item) => ChannelValue::HeatingDemand(item, value.into()),
            Channel::ExternalAutoControl(item) => {
                ChannelValue::ExternalAutoControl(item, value.into())
            }
            Channel::Presence(item) => ChannelValue::Presence(item, value.into()),
            Channel::TotalRadiatorConsumption(item) => {
                ChannelValue::TotalRadiatorConsumption(item, value.into())
            }
            Channel::TotalWaterConsumption(item) => {
                ChannelValue::TotalWaterConsumption(item, value.into())
            }
            Channel::FanActivity(item) => ChannelValue::FanActivity(item, value.into()),
        }
    }
}

#[derive(Debug, Clone, Hash, Eq, PartialEq, Id, EnumVariants)]
//TODO remove EnumVariants, only for state-debug
pub enum Temperature {
    Outside,
    LivingRoomDoor,
    RoomOfRequirementsDoor,
    BedroomDoor,
    BedroomOuterWall,
    KitchenOuterWall,
    BathroomShower,
    Dehumidifier,
}

#[derive(Debug, Clone, Hash, Eq, PartialEq, Id, EnumVariants)]
pub enum RelativeHumidity {
    Outside,
    LivingRoomDoor,
    RoomOfRequirementsDoor,
    BedroomDoor,
    BedroomOuterWall,
    KitchenOuterWall,
    BathroomShower,
    Dehumidifier,
}

#[derive(Debug, Clone, Hash, Eq, PartialEq, Id)]
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

#[derive(Debug, Clone, Hash, Eq, PartialEq, Id)]
pub enum Powered {
    Dehumidifier,
    LivingRoomNotificationLight,
    InfraredHeater,
    LivingRoomTv,
}

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

#[derive(Debug, Clone, Hash, Eq, PartialEq, Id)]
pub enum SetPoint {
    LivingRoom,
    Bedroom,
    Kitchen,
    RoomOfRequirements,
    Bathroom,
}

#[derive(Debug, Clone, Hash, Eq, PartialEq, Id, EnumVariants)]
pub enum HeatingDemand {
    LivingRoom,
    Bedroom,
    Kitchen,
    RoomOfRequirements,
    Bathroom,
}

#[derive(Debug, Clone, Hash, Eq, PartialEq, Id)]
pub enum ExternalAutoControl {
    LivingRoomThermostat,
    BedroomThermostat,
    KitchenThermostat,
    RoomOfRequirementsThermostat,
    BathroomThermostat,
}

#[derive(Debug, Clone, Hash, Eq, PartialEq, Id)]
pub enum Presence {
    AtHomeDennis,
    AtHomeSabine,
    BedDennis,
    BedSabine,
    CouchLeft,
    CouchCenter,
    CouchRight,
}

#[derive(Debug, Clone, Hash, Eq, PartialEq, Id)]
pub enum TotalRadiatorConsumption {
    LivingRoomBig,
    LivingRoomSmall,
    Bedroom,
    Kitchen,
    RoomOfRequirements,
    Bathroom,
}

#[derive(Debug, Clone, Hash, Eq, PartialEq, Id)]
pub enum TotalWaterConsumption {
    KitchenCold,
    KitchenWarm,
    BathroomCold,
    BathroomWarm,
}

#[derive(Debug, Clone, Hash, Eq, PartialEq, Id)]
pub enum FanActivity {
    LivingRoomCeilingFan,
}

pub mod unit {
    use std::fmt::Display;

    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
    pub enum FanAirflow {
        Off,
        Forward(FanSpeed),
        Reverse(FanSpeed),
    }

    #[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
    pub enum FanSpeed {
        Silent,
        Low,
        Medium,
        High,
        Turbo,
    }

    impl Display for FanAirflow {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                FanAirflow::Off => write!(f, "off"),
                FanAirflow::Forward(FanSpeed::Silent) => write!(f, "silent (fwd)"),
                FanAirflow::Forward(FanSpeed::Low) => write!(f, "low (fwd)"),
                FanAirflow::Forward(FanSpeed::Medium) => write!(f, "medium (fwd)"),
                FanAirflow::Forward(FanSpeed::High) => write!(f, "high (fwd)"),
                FanAirflow::Forward(FanSpeed::Turbo) => write!(f, "turbo (fwd)"),
                FanAirflow::Reverse(FanSpeed::Silent) => write!(f, "silent (rev)"),
                FanAirflow::Reverse(FanSpeed::Low) => write!(f, "low (rev)"),
                FanAirflow::Reverse(FanSpeed::Medium) => write!(f, "medium (rev)"),
                FanAirflow::Reverse(FanSpeed::High) => write!(f, "high (rev)"),
                FanAirflow::Reverse(FanSpeed::Turbo) => write!(f, "turbo (rev)"),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_display() {
        assert_eq!(
            RelativeHumidity::Outside.to_string(),
            "RelativeHumidity[Outside]"
        );
    }
}
