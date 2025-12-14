use crate::home::{HeatingZone, Room};

pub trait DashboardDisplay {
    fn display(&self) -> &'static str;
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

impl DashboardDisplay for HeatingZone {
    fn display(&self) -> &'static str {
        match self {
            HeatingZone::LivingRoom => "Wohnzimmer",
            HeatingZone::Bedroom => "Schlafzimmer",
            HeatingZone::Kitchen => "Küche",
            HeatingZone::RoomOfRequirements => "Room of Requirements",
            HeatingZone::Bathroom => "Bad",
        }
    }
}
