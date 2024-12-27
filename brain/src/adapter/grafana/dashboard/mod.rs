use api::state::HeatingDemand;
use serde::{Deserialize, Serialize};

use super::display::DashboardDisplay;

pub mod energy_iq;
pub mod energy_monitor;
pub mod meta;
pub mod state_debug;

use super::support::csv_response;
use super::support::empty_string_as_none;

const EURO_PER_KWH: f64 = 0.349;

fn heating_factor(item: &HeatingDemand) -> f64 {
    match item {
        HeatingDemand::LivingRoom => 1.728 + 0.501,
        HeatingDemand::Bedroom => 1.401,
        HeatingDemand::RoomOfRequirements => 1.193,
        HeatingDemand::Kitchen => 1.485,
        HeatingDemand::Bathroom => 0.496,
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
}
