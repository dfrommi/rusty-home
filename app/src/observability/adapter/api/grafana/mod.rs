pub mod meta;
pub mod overview;

use std::sync::Arc;

use crate::core::time::DateTime;
use crate::core::time::DateTimeRange;

use crate::{command::CommandClient, device_state::DeviceStateClient};
use actix_web::{HttpResponse, http::header};
use actix_web::{
    ResponseError,
    web::{self},
};
use anyhow::Context;
use derive_more::derive::{Display, Error};

type GrafanaResponse = Result<HttpResponse, GrafanaApiError>;

pub fn routes(command_client: Arc<CommandClient>, device_state_client: Arc<DeviceStateClient>) -> actix_web::Scope {
    web::scope("/grafana")
        .service(overview::routes(command_client, device_state_client))
        .service(meta::routes())
}

#[derive(Clone, Debug, serde::Deserialize)]
struct TimeRangeQuery {
    from: DateTime,
    to: DateTime,
}

impl TimeRangeQuery {
    fn range(&self) -> DateTimeRange {
        DateTimeRange::new(self.from, self.to).non_future()
    }
}

#[derive(Debug, Error, Display)]
enum GrafanaApiError {
    #[display("Error accessing data")]
    DataAccessError(anyhow::Error),

    #[display("Internal error")]
    InternalError(anyhow::Error),
}

impl ResponseError for GrafanaApiError {
    fn status_code(&self) -> actix_web::http::StatusCode {
        use actix_web::http::StatusCode;

        tracing::warn!("GrafanaApiError: {:?}", self);

        match self {
            GrafanaApiError::DataAccessError(_) | GrafanaApiError::InternalError(_) => {
                StatusCode::INTERNAL_SERVER_ERROR
            }
        }
    }
}

fn csv_response<S: serde::Serialize>(rows: impl IntoIterator<Item = S>) -> GrafanaResponse {
    let mut writer = csv::Writer::from_writer(vec![]);

    for row in rows {
        writer
            .serialize(row)
            .context("Error serializing row to CSV")
            .map_err(GrafanaApiError::InternalError)?;
    }

    let csv = writer
        .into_inner()
        .context("Error creating CSV")
        .map_err(GrafanaApiError::InternalError)?;

    Ok(HttpResponse::Ok()
        .append_header(header::ContentType(mime::TEXT_CSV))
        .body(csv))
}
