use support::unit::{
    DegreeCelsius, KiloWattHours, OpenedState, Percent, PowerState, PresentState,
    UserControlledState, Watt,
};

pub mod db;

/**
* TODO:
* [X] valve open state: "sensor.${room}_heating" (heating demand in percent)
* [X] target temperature: climate.${room}
* - (optional): manual control on/off: binary_sensor.${room}_overlay
*
* [X] ir heater energy consumption
*
* - water usage
* - heating consumption
*
* [X] presence bed dennis, sabine
* [X] presence couch
*
* - notification light (off, info, warn, alert)
* [X] home state: dennis, sabine
*
* - tv energy saving mode
*/

#[derive(Debug, Clone)]
pub enum ChannelValue {
    Temperature(Temperature, DegreeCelsius),
    RelativeHumidity(RelativeHumidity, Percent),
    Opened(Opened, OpenedState),
    Powered(Powered, PowerState),
    CurrentPowerUsage(CurrentPowerUsage, Watt),
    TotalEnergyConsumption(TotalEnergyConsumption, KiloWattHours),
    SetPoint(SetPoint, DegreeCelsius),
    HeatingDemand(HeatingDemand, Percent),
    UserControlled(UserControlled, UserControlledState),
    Presence(Presence, PresentState),
}

pub trait ChannelId {
    type ValueType;
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

impl ChannelId for Temperature {
    type ValueType = DegreeCelsius;
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

impl ChannelId for RelativeHumidity {
    type ValueType = Percent;
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

impl ChannelId for Opened {
    type ValueType = OpenedState;
}

#[derive(Debug, Clone, Hash, Eq, PartialEq, strum::IntoStaticStr)]
#[strum(serialize_all = "snake_case")]
pub enum Powered {
    Dehumidifier,
}

impl ChannelId for Powered {
    type ValueType = PowerState;
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
    InfraredHeater,
}

impl ChannelId for CurrentPowerUsage {
    type ValueType = Watt;
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
    InfraredHeater,
}

impl ChannelId for TotalEnergyConsumption {
    type ValueType = KiloWattHours;
}

#[derive(Debug, Clone, Hash, Eq, PartialEq, strum::IntoStaticStr)]
#[strum(serialize_all = "snake_case")]
pub enum SetPoint {
    LivingRoom,
    Bedroom,
    Kitchen,
    RoomOfRequirements,
    Bathroom,
}

impl ChannelId for SetPoint {
    type ValueType = DegreeCelsius;
}

#[derive(Debug, Clone, Hash, Eq, PartialEq, strum::IntoStaticStr)]
#[strum(serialize_all = "snake_case")]
pub enum HeatingDemand {
    LivingRoom,
    Bedroom,
    Kitchen,
    RoomOfRequirements,
    Bathroom,
}

impl ChannelId for HeatingDemand {
    type ValueType = Percent;
}

#[derive(Debug, Clone, Hash, Eq, PartialEq, strum::IntoStaticStr)]
#[strum(serialize_all = "snake_case")]
pub enum UserControlled {
    LivingRoomThermostat,
    BedroomThermostat,
    KitchenThermostat,
    RoomOfRequirementsThermostat,
    BathroomThermostat,
}

impl ChannelId for UserControlled {
    type ValueType = UserControlledState;
}

#[derive(Debug, Clone, Hash, Eq, PartialEq, strum::IntoStaticStr)]
#[strum(serialize_all = "snake_case")]
pub enum Presence {
    AtHomeDennis,
    AtHomeSabine,
    BedDennis,
    BedSabine,
    CouchLeft,
    CouchCenter,
    CouchRight,
}

impl ChannelId for Presence {
    type ValueType = PresentState;
}
