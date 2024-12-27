mod dashboard;
mod display;
mod support;

use std::sync::Arc;

use actix_web::{
    http::header,
    web::{self},
    HttpResponse, ResponseError,
};
use anyhow::Context;
use api::state::{
    CurrentPowerUsage, HeatingDemand, RelativeHumidity, Temperature, TotalEnergyConsumption,
};
use derive_more::derive::{Display, Error};

use crate::port::{DataPointAccess, TimeSeriesAccess};

use display::DashboardDisplay;

pub fn new_routes<T>(api: Arc<T>) -> actix_web::Scope
where
    T: DataPointAccess<CurrentPowerUsage>
        + DataPointAccess<HeatingDemand>
        + TimeSeriesAccess<TotalEnergyConsumption>
        + TimeSeriesAccess<HeatingDemand>
        + TimeSeriesAccess<Temperature>
        + TimeSeriesAccess<RelativeHumidity>
        + 'static,
{
    web::scope("/grafana")
        .service(dashboard::energy_iq::routes(api.clone()))
        .service(dashboard::energy_monitor::routes(api.clone()))
        .service(dashboard::state_debug::routes(api.clone()))
        .service(dashboard::meta::routes())
}

type GrafanaResponse = Result<HttpResponse, GrafanaApiError>;

#[derive(Debug, Error, Display)]
enum GrafanaApiError {
    #[display("Channel not found: type={_0} / name={_1}")]
    ChannelNotFound(String, String),

    #[display("Channel not supported: type={_0} / name={_1}")]
    ChannelUnsupported(String, String),

    #[display("Error accessing data")]
    DataAccessError(anyhow::Error),

    #[display("Internal error")]
    InternalError(anyhow::Error),
}

impl ResponseError for GrafanaApiError {
    fn status_code(&self) -> actix_web::http::StatusCode {
        use actix_web::http::StatusCode;
        match self {
            GrafanaApiError::ChannelNotFound(_, _) => StatusCode::NOT_FOUND,
            GrafanaApiError::ChannelUnsupported(_, _) => StatusCode::BAD_REQUEST,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}
