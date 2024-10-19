use support::unit::DegreeCelsius;

use crate::thing::planning::action::HeatingZone;

use super::action::{
    Dehumidify, Heat, HomeAction, NoHeatingDuringAutomaticTemperatureIncrease,
    NoHeatingDuringVentilation, RequestClosingWindow,
};
use super::goal::{HomeGoal, Room, RoomComfortLevel};

#[rustfmt::skip]
pub fn default_config() -> Vec<(HomeGoal, Vec<HomeAction>)> {
    let mut result = vec![
    (
        HomeGoal::AvoidUselessHeating,
        vec![
            HomeAction::NoHeatingDuringVentilation(NoHeatingDuringVentilation::new(HeatingZone::LivingRoom)),
            HomeAction::NoHeatingDuringVentilation(NoHeatingDuringVentilation::new(HeatingZone::Bedroom)),
            HomeAction::NoHeatingDuringVentilation(NoHeatingDuringVentilation::new(HeatingZone::Kitchen)),
            HomeAction::NoHeatingDuringVentilation(NoHeatingDuringVentilation::new(HeatingZone::RoomOfRequirements)),
            HomeAction::NoHeatingDuringVentilation(NoHeatingDuringVentilation::new(HeatingZone::Bathroom)),
            HomeAction::NoHeatingDuringAutomaticTemperatureIncrease(NoHeatingDuringAutomaticTemperatureIncrease::new(HeatingZone::LivingRoom)),
            HomeAction::NoHeatingDuringAutomaticTemperatureIncrease(NoHeatingDuringAutomaticTemperatureIncrease::new(HeatingZone::Bedroom)),
            HomeAction::NoHeatingDuringAutomaticTemperatureIncrease(NoHeatingDuringAutomaticTemperatureIncrease::new(HeatingZone::Kitchen)),
            HomeAction::NoHeatingDuringAutomaticTemperatureIncrease(NoHeatingDuringAutomaticTemperatureIncrease::new(HeatingZone::RoomOfRequirements)),
            HomeAction::NoHeatingDuringAutomaticTemperatureIncrease(NoHeatingDuringAutomaticTemperatureIncrease::new(HeatingZone::Bathroom)),
        ]
    ),
    (
        HomeGoal::StayInformed,
        vec![
            HomeAction::RequestClosingWindow(RequestClosingWindow {})
        ],
    ),
    (
        HomeGoal::PreventMouldInBathroom,
        vec![
            HomeAction::Dehumidify(Dehumidify {})
        ],
    ),
    ];

    for (room, level, temperature) in [
        (Room::LivingRoom, RoomComfortLevel::EnergySaving, 19.0),
        (Room::LivingRoom, RoomComfortLevel::Normal, 20.0),
        (Room::LivingRoom, RoomComfortLevel::Comfortable, 21.0),
        (Room::Bedroom, RoomComfortLevel::EnergySaving, 19.0),
        (Room::Bedroom, RoomComfortLevel::Normal, 20.0),
        (Room::Bedroom, RoomComfortLevel::Comfortable, 21.0),
        (Room::Kitchen, RoomComfortLevel::EnergySaving, 19.0),
        (Room::Kitchen, RoomComfortLevel::Normal, 20.0),
        (Room::Kitchen, RoomComfortLevel::Comfortable, 21.0),
        (Room::RoomOfRequirements, RoomComfortLevel::EnergySaving, 19.0),
        (Room::RoomOfRequirements, RoomComfortLevel::Normal, 20.0),
        (Room::RoomOfRequirements, RoomComfortLevel::Comfortable, 21.0),
        (Room::Bathroom, RoomComfortLevel::EnergySaving, 19.0),
        (Room::Bathroom, RoomComfortLevel::Normal, 20.0),
        (Room::Bathroom, RoomComfortLevel::Comfortable, 21.0),
    ] {
        let heating_zone = HeatingZone::from(&room);
        result.push((
            HomeGoal::RoomComfort(room, level),
            vec![
                HomeAction::Heat(Heat::new(heating_zone, DegreeCelsius(temperature)))
            ],
        ));
    }

    result
}

impl From<&Room> for HeatingZone {
    fn from(val: &Room) -> Self {
        match val {
            Room::LivingRoom => HeatingZone::LivingRoom,
            Room::Bedroom => HeatingZone::Bedroom,
            Room::Kitchen => HeatingZone::Kitchen,
            Room::RoomOfRequirements => HeatingZone::RoomOfRequirements,
            Room::Bathroom => HeatingZone::Bathroom,
        }
    }
}
