use crate::{
    adapter::metrics_export::MetricLabel,
    core::id::ExternalId,
    device_state::{CurrentPowerUsage, HeatingDemand, TotalEnergyConsumption},
    device_state::{DeviceStateId, DeviceStateValue},
};

pub fn get_common_tags(external_id: &ExternalId) -> Vec<MetricLabel> {
    let variant_name = external_id.variant_name();

    let mut tags = vec![MetricLabel::Variant(variant_name.to_owned())];

    if let Some(room_name) = room(external_id) {
        tags.push(MetricLabel::Room(room_name.to_owned()));
    }

    tags
}

pub fn get_tags_for_device(value: &DeviceStateValue) -> Vec<MetricLabel> {
    let mut tags = vec![];

    if let Some(friendly_name) = friendly_name(value.into()) {
        tags.push(MetricLabel::FriendlyName(friendly_name.to_owned()));
    }

    tags
}

fn room(ext_id: &ExternalId) -> Option<&'static str> {
    let variant_name = ext_id.variant_name().to_owned();

    if variant_name.contains("living_room") {
        Some("Wohnzimmer")
    } else if variant_name.contains("bedroom") {
        Some("Schlafzimmer")
    } else if variant_name.contains("kitchen") {
        Some("Küche")
    } else if variant_name.contains("room_of_requirements") {
        Some("Room of Requirements")
    } else if variant_name.contains("bathroom") {
        Some("Bad")
    } else {
        None
    }
}

fn friendly_name(state: DeviceStateId) -> Option<&'static str> {
    match state {
        DeviceStateId::CurrentPowerUsage(s) => Some(match s {
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
            CurrentPowerUsage::RoomOfRequirementsMonitor => "Monitor",
        }),
        DeviceStateId::TotalEnergyConsumption(s) => Some(match s {
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
            TotalEnergyConsumption::RoomOfRequirementsMonitor => "Monitor",
        }),
        DeviceStateId::HeatingDemand(s) => Some(match s {
            HeatingDemand::LivingRoomBig => "Wohnzimmer (groß)",
            HeatingDemand::LivingRoomSmall => "Wohnzimmer (klein)",
            HeatingDemand::Bedroom => "Schlafzimmer",
            HeatingDemand::RoomOfRequirements => "Room of Requirements",
            HeatingDemand::Kitchen => "Küche",
            HeatingDemand::Bathroom => "Bad",
        }),
        _ => None,
    }
}
