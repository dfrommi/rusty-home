mod dashboard;
mod display;
mod support;

use std::sync::Arc;

use ::support::ExternalId;
use actix_web::{
    HttpResponse, ResponseError,
    web::{self},
};
use derive_more::derive::{Display, Error};

use display::DashboardDisplay;

pub fn new_routes(api: crate::Database) -> actix_web::Scope {
    let api = Arc::new(api);

    web::scope("/grafana")
        .service(dashboard::energy_iq::routes(api.clone()))
        .service(dashboard::energy_monitor::routes(api.clone()))
        .service(dashboard::state_debug::routes(api.clone()))
        .service(dashboard::heating_details::routes(api.clone()))
        .service(dashboard::smart_home::routes(api.clone()))
        .service(dashboard::meta::routes())
}

type GrafanaResponse = Result<HttpResponse, GrafanaApiError>;

#[derive(Debug, Error, Display)]
enum GrafanaApiError {
    #[display("Channel not found: {_0}")]
    ChannelNotFound(#[error(not(source))] ExternalId),

    #[display("Channel not supported: {_0}")]
    ChannelUnsupported(#[error(not(source))] ExternalId),

    #[display("Error accessing data")]
    DataAccessError(anyhow::Error),

    #[display("Internal error")]
    InternalError(anyhow::Error),

    #[display("Not found")]
    NotFound,
}

impl ResponseError for GrafanaApiError {
    fn status_code(&self) -> actix_web::http::StatusCode {
        use actix_web::http::StatusCode;

        tracing::warn!("GrafanaApiError: {:?}", self);

        match self {
            GrafanaApiError::ChannelNotFound(_) => StatusCode::NOT_FOUND,
            GrafanaApiError::NotFound => StatusCode::NOT_FOUND,
            GrafanaApiError::ChannelUnsupported(_) => StatusCode::BAD_REQUEST,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}
