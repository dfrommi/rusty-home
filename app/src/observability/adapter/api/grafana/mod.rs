pub mod meta;
pub mod overview;

use crate::core::time::DateTime;
use crate::core::time::DateTimeRange;

use actix_web::{HttpResponse, http::header};
use anyhow::Context;
use serde::{Deserialize, Deserializer, de::IntoDeserializer};

use crate::{command::CommandClient, device_state::DeviceStateClient};
use actix_web::{
    ResponseError,
    web::{self},
};
use derive_more::derive::{Display, Error};

type GrafanaResponse = Result<HttpResponse, GrafanaApiError>;

pub fn routes(command_client: CommandClient, device_state_client: DeviceStateClient) -> actix_web::Scope {
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

/// To be used like this:
/// `#[serde(deserialize_with = "empty_string_as_none")]`
/// Relevant serde issue: <https://github.com/serde-rs/serde/issues/1425>
fn empty_string_as_none<'de, D, T>(de: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    let opt = Option::<String>::deserialize(de)?;
    match opt.as_deref() {
        None | Some("") => Ok(None),
        Some(s) => T::deserialize(s.into_deserializer()).map(Some),
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
