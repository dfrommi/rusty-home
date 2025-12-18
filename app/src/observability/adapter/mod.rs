pub mod api;
pub mod device_metrics;
pub mod home_metrics;
pub mod repository;

use super::domain::*;

use crate::core::id::ExternalId;

pub trait MetricsAdapter<T> {
    fn to_metrics(&self, item: T) -> Vec<Metric>;
}

pub fn get_common_tags(external_id: &ExternalId) -> Vec<MetricLabel> {
    let variant_name = external_id.variant_name();

    let mut tags = vec![MetricLabel::Variant(variant_name.to_owned())];

    if let Some(room_name) = room(external_id) {
        tags.push(MetricLabel::Room(room_name.to_owned()));
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
