use crate::{
    adapter::metrics_export::MetricLabel,
    core::id::ExternalId,
    home::state::{CurrentPowerUsage, HeatingDemand, HomeState, HomeStateValue, TotalEnergyConsumption},
};

pub fn get_tags(value: &HomeStateValue) -> Vec<MetricLabel> {
    let state: HomeState = value.into();
    let external_id = state.ext_id();
    let variant_name = external_id.variant_name();

    let mut tags = vec![MetricLabel::Variant(variant_name.to_owned())];

    if let Some(room_name) = room(state.ext_id()) {
        tags.push(MetricLabel::Room(room_name.to_owned()));
    }

    if let Some(friendly_name) = friendly_name(&state) {
        tags.push(MetricLabel::FriendlyName(friendly_name.to_owned()));
    }

    if let Some(enum_variant) = enum_variant(value) {
        tags.push(MetricLabel::EnumVariant(enum_variant));
    }

    tags
}

fn room(ext_id: ExternalId) -> Option<&'static str> {
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

fn friendly_name(state: &HomeState) -> Option<&'static str> {
    match state {
        HomeState::CurrentPowerUsage(s) => Some(match s {
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
        HomeState::TotalEnergyConsumption(s) => Some(match s {
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
        HomeState::HeatingDemand(s) => Some(match s {
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

fn enum_variant(value: &HomeStateValue) -> Option<String> {
    match value {
        HomeStateValue::ScheduledHeatingMode(_, v) => Some(v.ext_id().variant_name().to_string()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enum_variant_scheduled_heating_mode() {
        let value = HomeStateValue::ScheduledHeatingMode(
            crate::home::state::ScheduledHeatingMode::LivingRoom,
            crate::home::state::HeatingMode::Comfort,
        );

        let variant = enum_variant(&value);

        assert_eq!(variant, Some("comfort".to_string()));
    }
}
