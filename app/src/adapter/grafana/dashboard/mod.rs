use crate::core::time::DateTime;
use crate::core::time::DateTimeRange;
use crate::core::time::Duration;
use crate::home::HeatingZone;
use crate::home::state::HeatingDemand;
use crate::home::state::SetPoint;
use crate::home::state::Temperature;
use serde::{Deserialize, Serialize};

use crate::home::state::OpenedArea;

use super::display::DashboardDisplay;

pub mod energy_iq;
pub mod energy_monitor;
pub mod heating_details;
pub mod meta;
pub mod smart_home;

use super::support::empty_string_as_none;

const EURO_PER_KWH: f64 = 0.349;

#[derive(Clone, Debug, serde::Deserialize)]
struct TimeRangeQuery {
    from: DateTime,
    to: DateTime,
}

impl TimeRangeQuery {
    fn range(&self) -> DateTimeRange {
        DateTimeRange::new(self.from, self.to).non_future()
    }
}

#[derive(Clone, Debug, serde::Deserialize)]
struct TimeRangeWithIntervalQuery {
    from: DateTime,
    to: DateTime,
    interval_ms: i64,
}

impl TimeRangeWithIntervalQuery {
    fn range(&self) -> DateTimeRange {
        DateTimeRange::new(self.from, self.to).non_future()
    }

    fn iter(&self) -> impl Iterator<Item = DateTime> + '_ {
        self.range().step_by(Duration::millis(self.interval_ms))
    }
}

//TODO blanket impl for serde for TypedItem
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Room {
    LivingRoom,
    Bedroom,
    Kitchen,
    RoomOfRequirements,
    Bathroom,
}

//TODO derive and ensure serde-compat
impl Room {
    pub fn variants() -> &'static [Self] {
        &[
            Room::LivingRoom,
            Room::Bedroom,
            Room::Kitchen,
            Room::RoomOfRequirements,
            Room::Bathroom,
        ]
    }
}

impl DashboardDisplay for Room {
    fn display(&self) -> &'static str {
        match self {
            Room::LivingRoom => "Wohnzimmer",
            Room::Bedroom => "Schlafzimmer",
            Room::Kitchen => "Küche",
            Room::RoomOfRequirements => "Room of Requirements",
            Room::Bathroom => "Bad",
        }
    }
}

impl Room {
    fn heating_zone(&self) -> HeatingZone {
        match self {
            Room::LivingRoom => HeatingZone::LivingRoom,
            Room::Bedroom => HeatingZone::Bedroom,
            Room::Kitchen => HeatingZone::Kitchen,
            Room::RoomOfRequirements => HeatingZone::RoomOfRequirements,
            Room::Bathroom => HeatingZone::Bathroom,
        }
    }

    fn inside_temperature(&self) -> Temperature {
        match self {
            Room::LivingRoom => Temperature::LivingRoom,
            Room::Bedroom => Temperature::Bedroom,
            Room::Kitchen => Temperature::Kitchen,
            Room::RoomOfRequirements => Temperature::RoomOfRequirements,
            Room::Bathroom => Temperature::BathroomShower,
        }
    }

    fn window(&self) -> OpenedArea {
        match self {
            Room::LivingRoom => OpenedArea::LivingRoomWindowOrDoor,
            Room::Bedroom => OpenedArea::BedroomWindow,
            Room::Kitchen => OpenedArea::KitchenWindow,
            Room::RoomOfRequirements => OpenedArea::RoomOfRequirementsWindow,
            Room::Bathroom => OpenedArea::BedroomWindow,
        }
    }
}
