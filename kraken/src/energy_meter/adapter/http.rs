use std::sync::Arc;

use axum::{extract::State, http::StatusCode, routing::put, Json, Router};
use serde::Deserialize;

use super::AddEnergyReadingUseCase;
use super::{EnergyReading, Faucet, Radiator};

pub fn router<R>(add_reading: R) -> Router
where
    R: AddEnergyReadingUseCase + Clone + Send + Sync + 'static,
{
    let app_state = AppState {
        add_reading: Arc::new(add_reading),
    };

    Router::new()
        .route("/api/energy/readings/heating", put(handle_heating_reading))
        .route("/api/energy/readings/water", put(handle_water_reading))
        .with_state(app_state)
}

#[derive(Clone)]
struct AppState<R>
where
    R: AddEnergyReadingUseCase,
{
    add_reading: Arc<R>,
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
    R: AddEnergyReadingUseCase,
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

    if let Err(e) = state.add_reading.add_energy_reading(reading).await {
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
    R: AddEnergyReadingUseCase,
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

    if let Err(e) = state.add_reading.add_energy_reading(reading).await {
        tracing::error!("Error adding energy reading {:?}: {:?}", dto, e);
        return StatusCode::UNPROCESSABLE_ENTITY;
    }

    StatusCode::NO_CONTENT
}
