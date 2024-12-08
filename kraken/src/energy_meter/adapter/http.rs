use actix_web::web::{self, Json};
use actix_web::{HttpResponse, Responder};
use serde::Deserialize;

use super::AddEnergyReadingUseCase;
use super::{EnergyReading, Faucet, Radiator};

pub fn new_actix_web_scope<R>(add_reading: R) -> actix_web::Scope
where
    R: AddEnergyReadingUseCase + 'static,
{
    web::scope("/api/energy/readings")
        .route("/heating", web::put().to(handle_heating_reading::<R>))
        .route("/water", web::put().to(handle_water_reading::<R>))
        .app_data(web::Data::new(add_reading))
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

async fn handle_heating_reading<R>(
    api: web::Data<R>,
    Json(dto): Json<HeatingReadingDTO>,
) -> impl Responder
where
    R: AddEnergyReadingUseCase,
{
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

    if let Err(e) = api.add_energy_reading(reading).await {
        tracing::error!("Error adding energy reading {:?}: {:?}", dto, e);
        return HttpResponse::UnprocessableEntity();
    }

    HttpResponse::NoContent()
}

async fn handle_water_reading<R>(
    api: web::Data<R>,
    Json(dto): Json<WaterReadingDTO>,
) -> impl Responder
where
    R: AddEnergyReadingUseCase,
{
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

    if let Err(e) = api.add_energy_reading(reading).await {
        tracing::error!("Error adding energy reading {:?}: {:?}", dto, e);
        return HttpResponse::UnprocessableEntity();
    }

    HttpResponse::NoContent()
}
