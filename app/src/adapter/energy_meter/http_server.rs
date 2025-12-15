use crate::adapter::energy_meter::persistence::EnergyReadingRepository;
use crate::t;
use actix_web::web::{self, Json};
use actix_web::{HttpResponse, Responder};
use serde::Deserialize;
use tokio::sync::broadcast;

use super::{EnergyReading, EnergyReadingAddedEvent, Faucet, Radiator};

#[derive(Clone)]
struct EnergyMeterApiState {
    repo: EnergyReadingRepository,
    events: broadcast::Sender<EnergyReadingAddedEvent>,
}

pub fn new_actix_web_scope(
    repo: EnergyReadingRepository,
    events: broadcast::Sender<EnergyReadingAddedEvent>,
) -> actix_web::Scope {
    let state = EnergyMeterApiState { repo, events };
    web::scope("/api/energy/readings")
        .route("/heating", web::put().to(handle_heating_reading))
        .route("/water", web::put().to(handle_water_reading))
        .app_data(web::Data::new(state))
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
    state: web::Data<EnergyMeterApiState>,
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

    let id = match state.repo.add_yearly_energy_reading(reading, t!(now)).await {
        Ok(id) => id,
        Err(e) => {
            tracing::error!("Error adding energy reading {:?}: {:?}", dto, e);
            return HttpResponse::UnprocessableEntity();
        }
    };

    if let Err(e) = state.events.send(EnergyReadingAddedEvent { id }) {
        tracing::error!("Error adding energy reading {:?}: {:?}", dto, e);
    }

    HttpResponse::NoContent()
}

async fn handle_water_reading(
    state: web::Data<EnergyMeterApiState>,
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

    let id = match state.repo.add_yearly_energy_reading(reading, t!(now)).await {
        Ok(id) => id,
        Err(e) => {
            tracing::error!("Error adding energy reading {:?}: {:?}", dto, e);
            return HttpResponse::UnprocessableEntity();
        }
    };

    if let Err(e) = state.events.send(EnergyReadingAddedEvent { id }) {
        tracing::error!("Error adding energy reading {:?}: {:?}", dto, e);
    }

    HttpResponse::NoContent()
}
