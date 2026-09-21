use actix_web::web::{self, Json};
use actix_web::{HttpResponse, Responder};
use infrastructure::EventEmitter;
use serde::Deserialize;

use super::{EnergyReading, Radiator};

type EnergyReadingSender = EventEmitter<EnergyReading>;

pub fn new_actix_web_scope(events: EventEmitter<EnergyReading>) -> actix_web::Scope {
    web::scope("/api/energy/readings")
        .route("/heating", web::put().to(handle_heating_reading))
        .app_data(web::Data::new(events))
}

#[derive(Debug, Deserialize)]
struct HeatingReadingDTO {
    label: String,
    value: String,
}

async fn handle_heating_reading(
    sender: web::Data<EnergyReadingSender>,
    Json(dto): Json<HeatingReadingDTO>,
) -> impl Responder {
    let radiator = match dto.label.as_str() {
        "Wohnzimmer (groß)" => Radiator::LivingRoomBig,
        "Wohnzimmer (klein)" => Radiator::LivingRoomSmall,
        "Room of Requirements" => Radiator::RoomOfRequirements,
        "Küche" => Radiator::Kitchen,
        "Schlafzimmer" => Radiator::Bedroom,
        "Bad" => Radiator::Bathroom,
        _ => return HttpResponse::BadRequest(),
    };

    let value = match dto.value.parse::<f64>() {
        Ok(v) => v,
        Err(_) => return HttpResponse::BadRequest(),
    };

    let reading = EnergyReading::Heating(radiator, value);

    tracing::info!("Received reading {:?}", reading);

    sender.send(reading);

    HttpResponse::NoContent()
}
