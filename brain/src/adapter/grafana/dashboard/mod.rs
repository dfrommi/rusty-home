use api::state::HeatingDemand;
use api::state::SetPoint;
use api::state::Temperature;
use serde::{Deserialize, Serialize};
use support::time::DateTime;
use support::time::DateTimeRange;
use support::time::Duration;

use crate::home::state::Opened;

use super::display::DashboardDisplay;

pub mod energy_iq;
pub mod energy_monitor;
pub mod heating_details;
pub mod meta;
pub mod smart_home_overview;
pub mod state_debug;

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
    fn heating_demand(&self) -> HeatingDemand {
        match self {
            Room::LivingRoom => HeatingDemand::LivingRoom,
            Room::Bedroom => HeatingDemand::Bedroom,
            Room::Kitchen => HeatingDemand::Kitchen,
            Room::RoomOfRequirements => HeatingDemand::RoomOfRequirements,
            Room::Bathroom => HeatingDemand::Bathroom,
        }
    }

    fn heating_factor(&self) -> f64 {
        match self {
            Room::LivingRoom => 1.728 + 0.501,
            Room::Bedroom => 1.401,
            Room::Kitchen => 1.485,
            Room::RoomOfRequirements => 1.193,
            Room::Bathroom => 0.496,
        }
    }

    fn inside_temperature(&self) -> Temperature {
        match self {
            Room::LivingRoom => Temperature::LivingRoomDoor,
            Room::Bedroom => Temperature::BedroomDoor,
            Room::Kitchen => Temperature::KitchenOuterWall,
            Room::RoomOfRequirements => Temperature::RoomOfRequirementsDoor,
            Room::Bathroom => Temperature::BathroomShower,
        }
    }

    fn set_point(&self) -> SetPoint {
        match self {
            Room::LivingRoom => SetPoint::LivingRoom,
            Room::Bedroom => SetPoint::Bedroom,
            Room::Kitchen => SetPoint::Kitchen,
            Room::RoomOfRequirements => SetPoint::RoomOfRequirements,
            Room::Bathroom => SetPoint::Bathroom,
        }
    }

    fn window(&self) -> Opened {
        match self {
            Room::LivingRoom => Opened::LivingRoomWindowOrDoor,
            Room::Bedroom => Opened::BedroomWindow,
            Room::Kitchen => Opened::KitchenWindow,
            Room::RoomOfRequirements => Opened::RoomOfRequirementsWindow,
            Room::Bathroom => Opened::BedroomWindow,
        }
    }
}
