use api::command::{PowerToggle, Thermostat};
use support::t;
use support::unit::DegreeCelsius;

use crate::thing::planning::action::HeatingZone;
use crate::thing::UserControlled;

use super::action::{
    DeferHeatingUntilVentilationDone, Dehumidify, ExtendHeatingUntilSleeping, HomeAction,
    KeepUserOverride, NoHeatingDuringAutomaticTemperatureIncrease, NoHeatingDuringVentilation,
    RequestClosingWindow,
};
use super::goal::{HomeGoal, Room};

#[rustfmt::skip]
pub fn default_config() -> Vec<(HomeGoal, Vec<HomeAction>)> {
    vec![
    (
        HomeGoal::SmarterHeating(Room::LivingRoom),
        vec![
            NoHeatingDuringVentilation::new(HeatingZone::LivingRoom).into(),
            KeepUserOverride::new(UserControlled::LivingRoomThermostat, Thermostat::LivingRoom.into()).into(),
            NoHeatingDuringAutomaticTemperatureIncrease::new(HeatingZone::LivingRoom).into(),
            ExtendHeatingUntilSleeping::new(HeatingZone::LivingRoom, DegreeCelsius(20.0), t!(22:30-2:30)).into(),
            DeferHeatingUntilVentilationDone::new(HeatingZone::LivingRoom, DegreeCelsius(18.5), t!(6:12-12:30)).into(),
        ]
    ),
    (
        HomeGoal::SmarterHeating(Room::Bedroom),
        vec![
            NoHeatingDuringVentilation::new(HeatingZone::Bedroom).into(),
            KeepUserOverride::new(UserControlled::BedroomThermostat, Thermostat::Bedroom.into()).into(),
            NoHeatingDuringAutomaticTemperatureIncrease::new(HeatingZone::Bedroom).into(),
            ExtendHeatingUntilSleeping::new(HeatingZone::Bedroom, DegreeCelsius(19.0), t!(22:30-2:30)).into(),
            DeferHeatingUntilVentilationDone::new(HeatingZone::Bedroom, DegreeCelsius(18.0), t!(6:12-12:30)).into(),
        ]
    ),
    (
        HomeGoal::SmarterHeating(Room::Kitchen),
        vec![
            NoHeatingDuringVentilation::new(HeatingZone::Kitchen).into(),
            KeepUserOverride::new(UserControlled::KitchenThermostat, Thermostat::Kitchen.into()).into(),
            NoHeatingDuringAutomaticTemperatureIncrease::new(HeatingZone::Kitchen).into(),
            DeferHeatingUntilVentilationDone::new(HeatingZone::Kitchen, DegreeCelsius(15.0), t!(6:12-12:30)).into(),
        ]
    ),
    (
        HomeGoal::SmarterHeating(Room::RoomOfRequirements),
        vec![
            NoHeatingDuringVentilation::new(HeatingZone::RoomOfRequirements).into(),
            KeepUserOverride::new(UserControlled::RoomOfRequirementsThermostat, Thermostat::RoomOfRequirements.into()).into(),
            NoHeatingDuringAutomaticTemperatureIncrease::new(HeatingZone::RoomOfRequirements).into(),
        ]
    ),
    (
        HomeGoal::SmarterHeating(Room::Bathroom),
        vec![
            NoHeatingDuringVentilation::new(HeatingZone::Bathroom).into(),
            KeepUserOverride::new(UserControlled::BathroomThermostat, Thermostat::Bathroom.into()).into(),
            NoHeatingDuringAutomaticTemperatureIncrease::new(HeatingZone::Bathroom).into(),
        ]
    ),
    (
        HomeGoal::StayInformed,
        vec![
            RequestClosingWindow.into()
        ],
    ),
    (
        HomeGoal::PreventMouldInBathroom,
        vec![
            KeepUserOverride::new(UserControlled::Dehumidifier, PowerToggle::Dehumidifier.into()).into(),
            Dehumidify.into()
        ],
    ),
    ]
}
