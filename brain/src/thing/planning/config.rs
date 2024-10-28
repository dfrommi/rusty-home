use support::unit::DegreeCelsius;

use crate::thing::planning::action::HeatingZone;
use crate::thing::UserControlled;

use super::action::{
    DeferHeatingUntilVentilationDone, Dehumidify, ExtendHeatingUntilSleeping, HomeAction,
    KeepUserOverride, NoHeatingDuringAutomaticTemperatureIncrease, NoHeatingDuringVentilation,
    RequestClosingWindow, Resource,
};
use super::goal::{HomeGoal, Room};

#[rustfmt::skip]
pub fn default_config() -> Vec<(HomeGoal, Vec<HomeAction>)> {
    vec![
    (
        HomeGoal::SmarterHeating(Room::LivingRoom),
        vec![
            NoHeatingDuringVentilation::new(HeatingZone::LivingRoom).into(),
            KeepUserOverride::new(UserControlled::LivingRoomThermostat, Resource::LivingRoomThermostat).into(),
            NoHeatingDuringAutomaticTemperatureIncrease::new(HeatingZone::LivingRoom).into(),
            ExtendHeatingUntilSleeping::new(HeatingZone::LivingRoom, DegreeCelsius(19.1), (22,30), (2,30)).into(),
            DeferHeatingUntilVentilationDone::new(HeatingZone::LivingRoom, DegreeCelsius(17.6), (6,12), (12,30)).into(),
        ]
    ),
    (
        HomeGoal::SmarterHeating(Room::Bedroom),
        vec![
            NoHeatingDuringVentilation::new(HeatingZone::Bedroom).into(),
            KeepUserOverride::new(UserControlled::BedroomThermostat, Resource::BedroomThermostat).into(),
            NoHeatingDuringAutomaticTemperatureIncrease::new(HeatingZone::Bedroom).into(),
            ExtendHeatingUntilSleeping::new(HeatingZone::Bedroom, DegreeCelsius(18.6), (22,30), (2,30)).into(),
            DeferHeatingUntilVentilationDone::new(HeatingZone::Bedroom, DegreeCelsius(15.1), (6,12), (12,30)).into(),
        ]
    ),
    (
        HomeGoal::SmarterHeating(Room::Kitchen),
        vec![
            NoHeatingDuringVentilation::new(HeatingZone::Kitchen).into(),
            KeepUserOverride::new(UserControlled::KitchenThermostat, Resource::KitchenThermostat).into(),
            NoHeatingDuringAutomaticTemperatureIncrease::new(HeatingZone::Kitchen).into(),
            DeferHeatingUntilVentilationDone::new(HeatingZone::Kitchen, DegreeCelsius(15.1), (6,12), (12,30)).into(),
        ]
    ),
    (
        HomeGoal::SmarterHeating(Room::RoomOfRequirements),
        vec![
            NoHeatingDuringVentilation::new(HeatingZone::RoomOfRequirements).into(),
            KeepUserOverride::new(UserControlled::RoomOfRequirementsThermostat, Resource::RoomOfRequirementsThermostat).into(),
            NoHeatingDuringAutomaticTemperatureIncrease::new(HeatingZone::RoomOfRequirements).into(),
        ]
    ),
    (
        HomeGoal::SmarterHeating(Room::Bathroom),
        vec![
            NoHeatingDuringVentilation::new(HeatingZone::Bathroom).into(),
            KeepUserOverride::new(UserControlled::BathroomThermostat, Resource::BathroomThermostat).into(),
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
            KeepUserOverride::new(UserControlled::Dehumidifier, Resource::Dehumidifier).into(),
            Dehumidify.into()
        ],
    ),
    ]
}
