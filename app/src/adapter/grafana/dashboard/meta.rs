use actix_web::web;

use crate::{
    adapter::grafana::{GrafanaResponse, display::DashboardDisplay, support::csv_response},
    home::HeatingZone,
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
        .iter()
        .map(|room| Row {
            id: room.clone(),
            label: DashboardDisplay::display(room).to_string(),
        })
        .collect();

    csv_response(&rows)
}
