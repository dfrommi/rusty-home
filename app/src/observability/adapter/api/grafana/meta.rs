use actix_web::web;

use crate::{
    automation::{HeatingZone, Room},
    observability::adapter::api::grafana::{GrafanaResponse, csv_response},
};

pub fn routes() -> actix_web::Scope {
    web::scope("/meta").route("/room", web::get().to(get_rooms))
}

async fn get_rooms() -> GrafanaResponse {
    #[derive(serde::Serialize)]
    struct Row {
        id: HeatingZone,
        label: String,
    }

    let rows: Vec<Row> = HeatingZone::variants()
        .into_iter()
        .map(|zone| Row {
            id: zone.clone(),
            label: display_heating_zone(&zone).to_string(),
        })
        .collect();

    csv_response(&rows)
}

fn display_room(room: Room) -> &'static str {
    match room {
        Room::LivingRoom => "Wohnzimmer",
        Room::Bedroom => "Schlafzimmer",
        Room::Kitchen => "Küche",
        Room::RoomOfRequirements => "Room of Requirements",
        Room::Bathroom => "Bad",
    }
}

fn display_heating_zone(heating_zone: &HeatingZone) -> &'static str {
    match heating_zone {
        HeatingZone::LivingRoom => "Wohnzimmer",
        HeatingZone::Bedroom => "Schlafzimmer",
        HeatingZone::Kitchen => "Küche",
        HeatingZone::RoomOfRequirements => "Room of Requirements",
        HeatingZone::Bathroom => "Bad",
    }
}
