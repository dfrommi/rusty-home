mod dashboard;
mod display;
mod support;

use std::sync::Arc;

use crate::{core::HomeApi, device_state::DeviceStateClient};
use actix_web::{
    HttpResponse, ResponseError,
    web::{self},
};
use derive_more::derive::{Display, Error};

pub fn new_routes(api: HomeApi, device_state_client: DeviceStateClient) -> actix_web::Scope {
    let api = Arc::new(api);
    let device_state_client = Arc::new(device_state_client);

    web::scope("/grafana")
        .service(dashboard::smart_home::routes(api.clone(), device_state_client))
        .service(dashboard::meta::routes())
}

type GrafanaResponse = Result<HttpResponse, GrafanaApiError>;

#[derive(Debug, Error, Display)]
enum GrafanaApiError {
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
            GrafanaApiError::NotFound => StatusCode::NOT_FOUND,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}
