use actix_web::web::{self, Json};
use actix_web::{HttpResponse, Responder};
use serde::Deserialize;
use tokio::sync::mpsc;

use super::{EnergyReading, Faucet, Radiator};

type EnergyReadingSender = mpsc::Sender<EnergyReading>;

pub fn new_actix_web_scope(events: EnergyReadingSender) -> actix_web::Scope {
    web::scope("/api/energy/readings")
        .route("/heating", web::put().to(handle_heating_reading))
        .route("/water", web::put().to(handle_water_reading))
        .app_data(web::Data::new(events))
}

#[derive(Debug, Deserialize)]
struct HeatingReadingDTO {
    label: String,
    value: String,
}

#[derive(Debug, Deserialize)]
struct WaterReadingDTO {
    label: String,
    value: String,
    is_hot: bool,
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

    if let Err(e) = sender.send(reading).await {
        tracing::error!("Error sending energy reading {:?}: {:?}", dto, e);
        return HttpResponse::InternalServerError();
    }

    HttpResponse::NoContent()
}

async fn handle_water_reading(
    sender: web::Data<EnergyReadingSender>,
    Json(dto): Json<WaterReadingDTO>,
) -> impl Responder {
    let faucet = match dto.label.as_str() {
        "Küche" => Faucet::Kitchen,
        "Bad" => Faucet::Bathroom,
        _ => return HttpResponse::BadRequest(),
    };

    let value = match dto.value.parse::<f64>() {
        Ok(v) => v / 1000.0,
        Err(_) => return HttpResponse::BadRequest(),
    };

    let reading = if dto.is_hot {
        EnergyReading::HotWater(faucet, value)
    } else {
        EnergyReading::ColdWater(faucet, value)
    };

    tracing::info!("Adding reading {:?}", reading);

    if let Err(e) = sender.send(reading).await {
        tracing::error!("Error sending energy reading {:?}: {:?}", dto, e);
        return HttpResponse::InternalServerError();
    }

    HttpResponse::NoContent()
}
