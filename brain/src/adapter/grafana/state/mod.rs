use std::cmp::Ordering;

use actix_web::{
    web::{self, Query},
    Responder,
};
use anyhow::Context;
use api::state::{Channel, HeatingDemand, TotalEnergyConsumption};
use strum::VariantArray;
use support::{time::DateTime, DataPoint};

use crate::{
    adapter::grafana::csv_response, port::TimeSeriesAccess,
    support::timeseries::interpolate::Estimatable,
};

use super::{GrafanaApiError, QueryTimeRange};

#[derive(serde::Serialize)]
struct Row {
    timestamp: DateTime,
    #[serde(rename = "type")]
    type_: String,
    item: String,
    value: f64,
}

pub async fn get_types() -> impl Responder {
    let mut supported_channels: Vec<Channel> = vec![];
    supported_channels.extend(TotalEnergyConsumption::VARIANTS.iter().map(|c| c.into()));
    supported_channels.extend(HeatingDemand::VARIANTS.iter().map(|c| c.into()));

    csv_response(&supported_channels)
}

pub async fn get_items(path: web::Path<String>) -> impl Responder {
    let type_ = path.into_inner();

    let mut supported_channels: Vec<Channel> = vec![];
    supported_channels.extend(TotalEnergyConsumption::VARIANTS.iter().map(|c| c.into()));
    supported_channels.extend(HeatingDemand::VARIANTS.iter().map(|c| c.into()));

    let items = supported_channels.iter().filter_map(|c| {
        let channel_type = into_type_and_item(c.clone()).unwrap();
        if channel_type.0 == type_ {
            Some(channel_type.1)
        } else {
            None
        }
    });

    csv_response(&items.collect::<Vec<_>>())
}

pub async fn state_ts<T>(
    api: web::Data<T>,
    path: web::Path<(String, String)>,
    time_range: Query<QueryTimeRange>,
) -> Result<impl Responder, GrafanaApiError>
where
    T: TimeSeriesAccess<TotalEnergyConsumption> + TimeSeriesAccess<HeatingDemand>,
{
    let channel = into_channel(&path.0, &path.1)?;

    let mut rows = match channel {
        Channel::TotalEnergyConsumption(item) => get_rows(api.as_ref(), &item, &time_range).await,
        Channel::HeatingDemand(item) => get_rows(api.as_ref(), &item, &time_range).await,
        _ => return Err(GrafanaApiError::ChannelUnsupported(channel)),
    }?;

    rows.sort_by(|a, b| b.value.partial_cmp(&a.value).unwrap_or(Ordering::Equal));

    csv_response(&rows).map_err(|e| {
        tracing::error!("Error serializing response: {:?}", e);
        e
    })
}

async fn get_rows<T>(
    api: &impl TimeSeriesAccess<T>,
    item: &T,
    time_range: &QueryTimeRange,
) -> Result<Vec<Row>, GrafanaApiError>
where
    T: Estimatable + Clone + Into<Channel>,
    T::Type: AsRef<f64>,
{
    let ts = api
        .series(item.clone(), time_range.range())
        .await
        .map_err(GrafanaApiError::DataAccessError)?;

    let dps: Vec<DataPoint<<T>::Type>> = match time_range.interval() {
        Some(interval) => time_range
            .range()
            .step_by(interval)
            .filter_map(|t| ts.at(t))
            .collect::<Vec<_>>(),
        None => ts.inner().iter().cloned().collect(),
    };

    let (type_, item) =
        into_type_and_item(item.clone().into()).map_err(GrafanaApiError::InternalError)?;

    let rows: Vec<Row> = dps
        .iter()
        .map(|dp| Row {
            timestamp: dp.timestamp,
            type_: type_.to_string(),
            item: item.to_string(),
            value: *dp.value.as_ref(),
        })
        .collect();

    Ok(rows)
}

fn into_channel(type_: &str, item: &str) -> Result<Channel, GrafanaApiError> {
    let channel_json = serde_json::json!({
        "type": type_,
        "item": item,
    });

    serde_json::from_value::<Channel>(channel_json)
        .map_err(|_| GrafanaApiError::ChannelNotFound(type_.to_string(), item.to_string()))
}

fn into_type_and_item(channel: Channel) -> Result<(String, String), anyhow::Error> {
    let v = serde_json::to_value(channel).context("Error serializing channel to JSON")?;
    let type_ = v
        .get("type")
        .context("Channel does not contain type")?
        .as_str()
        .unwrap();
    let name = v
        .get("item")
        .context("Channel does not contain item")?
        .as_str()
        .unwrap();
    Ok((type_.to_string(), name.to_string()))
}
