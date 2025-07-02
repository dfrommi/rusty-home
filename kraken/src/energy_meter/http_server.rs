use actix_web::web::{self, Json};
use actix_web::{HttpResponse, Responder};
use serde::Deserialize;
use support::t;

use crate::Database;

use super::{EnergyReading, Faucet, Radiator};

pub fn new_actix_web_scope(api: Database) -> actix_web::Scope {
    web::scope("/api/energy/readings")
        .route("/heating", web::put().to(handle_heating_reading))
        .route("/water", web::put().to(handle_water_reading))
        .app_data(web::Data::new(api))
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
    api: web::Data<Database>,
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

    tracing::info!("Adding reading {:?}", reading);

    if let Err(e) = api.add_yearly_energy_reading(reading, t!(now)).await {
        tracing::error!("Error adding energy reading {:?}: {:?}", dto, e);
        return HttpResponse::UnprocessableEntity();
    }

    HttpResponse::NoContent()
}

async fn handle_water_reading(
    api: web::Data<Database>,
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

    if let Err(e) = api.add_yearly_energy_reading(reading, t!(now)).await {
        tracing::error!("Error adding energy reading {:?}: {:?}", dto, e);
        return HttpResponse::UnprocessableEntity();
    }

    HttpResponse::NoContent()
}
