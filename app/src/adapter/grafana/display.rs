use crate::home::{HeatingZone, Room, state::*};

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

impl DashboardDisplay for CurrentPowerUsage {
    fn display(&self) -> &'static str {
        match self {
            CurrentPowerUsage::Fridge => "Kühlschrank",
            CurrentPowerUsage::Dehumidifier => "Blasi",
            CurrentPowerUsage::AppleTv => "Apple TV",
            CurrentPowerUsage::Tv => "TV",
            CurrentPowerUsage::AirPurifier => "Luftfilter",
            CurrentPowerUsage::CouchLight => "Couchlicht",
            CurrentPowerUsage::Dishwasher => "Geschirrspüler",
            CurrentPowerUsage::Kettle => "Wasserkocher",
            CurrentPowerUsage::WashingMachine => "Waschmaschine",
            CurrentPowerUsage::Nuc => "Nuc",
            CurrentPowerUsage::DslModem => "DSL Modem",
            CurrentPowerUsage::InternetGateway => "Internet Gateway",
            CurrentPowerUsage::NetworkSwitch => "Network Switch",
            CurrentPowerUsage::InfraredHeater => "Infrarot-Heizung",
            CurrentPowerUsage::KitchenMultiPlug => "Küche Arbeitsplatte",
            CurrentPowerUsage::CouchPlug => "Couch-Stecker",
            CurrentPowerUsage::RoomOfRequirementsDesk => "Schreibtisch",
        }
    }
}

impl DashboardDisplay for TotalEnergyConsumption {
    fn display(&self) -> &'static str {
        match self {
            TotalEnergyConsumption::Fridge => "Kühlschrank",
            TotalEnergyConsumption::Dehumidifier => "Blasi",
            TotalEnergyConsumption::AppleTv => "Apple TV",
            TotalEnergyConsumption::Tv => "TV",
            TotalEnergyConsumption::AirPurifier => "Luftfilter",
            TotalEnergyConsumption::CouchLight => "Couchlicht",
            TotalEnergyConsumption::Dishwasher => "Geschirrspüler",
            TotalEnergyConsumption::Kettle => "Wasserkocher",
            TotalEnergyConsumption::WashingMachine => "Waschmaschine",
            TotalEnergyConsumption::Nuc => "Nuc",
            TotalEnergyConsumption::DslModem => "DSL Modem",
            TotalEnergyConsumption::InternetGateway => "Internet Gateway",
            TotalEnergyConsumption::NetworkSwitch => "Network Switch",
            TotalEnergyConsumption::InfraredHeater => "Infrarot-Heizung",
            TotalEnergyConsumption::KitchenMultiPlug => "Küche Arbeitsplatte",
            TotalEnergyConsumption::CouchPlug => "Couch-Stecker",
            TotalEnergyConsumption::RoomOfRequirementsDesk => "Schreibtisch",
        }
    }
}

impl DashboardDisplay for HeatingDemand {
    fn display(&self) -> &'static str {
        match self {
            HeatingDemand::LivingRoomBig => "Wohnzimmer (groß)",
            HeatingDemand::LivingRoomSmall => "Wohnzimmer (klein)",
            HeatingDemand::Bedroom => "Schlafzimmer",
            HeatingDemand::RoomOfRequirements => "Room of Requirements",
            HeatingDemand::Kitchen => "Küche",
            HeatingDemand::Bathroom => "Bad",
        }
    }
}
