use std::sync::Arc;

use axum::{extract::State, http::StatusCode, routing::put, Json, Router};
use serde::Deserialize;
use support::t;

use super::persistence::{EnergyReading, EnergyReadingRepository, Faucet, Radiator};

pub struct ManualEnergyMeter {}

impl ManualEnergyMeter {
    pub fn new<R>(repo: Arc<R>) -> Router
    where
        R: EnergyReadingRepository + Send + Clone + Sync + 'static,
    {
        let app_state = AppState { repository: repo };

        Router::new()
            .route("/api/energy/readings/heating", put(handle_heating_reading))
            .route("/api/energy/readings/water", put(handle_water_reading))
            .with_state(app_state)
    }
}

#[derive(Clone)]
struct AppState<R>
where
    R: EnergyReadingRepository,
{
    repository: Arc<R>,
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
    State(state): State<AppState<R>>,
    Json(dto): Json<HeatingReadingDTO>,
) -> StatusCode
where
    R: EnergyReadingRepository + Send + Clone + Sync,
{
    let radiator = match dto.label.as_str() {
        "Wohnzimmer (groß)" => Radiator::LivingRoomBig,
        "Wohnzimmer (klein)" => Radiator::LivingRoomSmall,
        "Room of Requirements" => Radiator::RoomOfRequirements,
        "Küche" => Radiator::Kitchen,
        "Schlafzimmer" => Radiator::Bedroom,
        "Bad" => Radiator::Bathroom,
        _ => return StatusCode::BAD_REQUEST,
    };

    let value = match dto.value.parse::<f64>() {
        Ok(v) => v,
        Err(_) => return StatusCode::BAD_REQUEST,
    };

    let reading = EnergyReading::Heating(radiator, value);

    tracing::info!("Adding reading {:?}", reading);

    if let Err(e) = state.repository.add_energy_reading(reading, t!(now)).await {
        tracing::error!("Error adding energy reading {:?}: {:?}", dto, e);
        return StatusCode::UNPROCESSABLE_ENTITY;
    }

    StatusCode::NO_CONTENT
}

async fn handle_water_reading<R>(
    State(state): State<AppState<R>>,
    Json(dto): Json<WaterReadingDTO>,
) -> StatusCode
where
    R: EnergyReadingRepository + Send + Clone + Sync,
{
    let faucet = match dto.label.as_str() {
        "Küche" => Faucet::Kitchen,
        "Bad" => Faucet::Bathroom,
        _ => return StatusCode::BAD_REQUEST,
    };

    let value = match dto.value.parse::<f64>() {
        Ok(v) => v / 1000.0,
        Err(_) => return StatusCode::BAD_REQUEST,
    };

    let reading = if dto.is_hot {
        EnergyReading::HotWater(faucet, value)
    } else {
        EnergyReading::ColdWater(faucet, value)
    };

    tracing::info!("Adding reading {:?}", reading);

    if let Err(e) = state.repository.add_energy_reading(reading, t!(now)).await {
        tracing::error!("Error adding energy reading {:?}: {:?}", dto, e);
        return StatusCode::UNPROCESSABLE_ENTITY;
    }

    StatusCode::NO_CONTENT
}
