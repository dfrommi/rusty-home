use actix_web::web;

use crate::{
    automation::Room,
    observability::adapter::api::grafana::{GrafanaResponse, csv_response},
};

pub fn routes() -> actix_web::Scope {
    web::scope("/meta").route("/room", web::get().to(get_rooms))
}

async fn get_rooms() -> GrafanaResponse {
    #[derive(serde::Serialize)]
    struct Row {
        id: Room,
        label: String,
    }

    let rows: Vec<Row> = Room::variants()
        .iter()
        .map(|room| Row {
            id: *room,
            label: display_room(*room).to_string(),
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
