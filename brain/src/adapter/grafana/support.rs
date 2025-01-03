use actix_web::{http::header, HttpResponse};
use anyhow::Context;
use serde::{de::IntoDeserializer, Deserialize, Deserializer};

use super::{GrafanaApiError, GrafanaResponse};

/// To be used like this:
/// `#[serde(deserialize_with = "empty_string_as_none")]`
/// Relevant serde issue: <https://github.com/serde-rs/serde/issues/1425>
pub fn empty_string_as_none<'de, D, T>(de: D) -> Result<Option<T>, D::Error>
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

pub fn csv_response<S: serde::Serialize>(rows: impl IntoIterator<Item = S>) -> GrafanaResponse {
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
