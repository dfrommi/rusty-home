use std::cmp::Ordering;

use actix_web::{
    web::{self, Query},
    Responder,
};
use api::state::{Channel, HeatingDemand, TotalEnergyConsumption};
use support::TypedItem;
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

#[derive(Debug, Clone, serde::Serialize)]
struct TypeAndItem {
    #[serde(rename = "type")]
    type_: String,
    item: String,
}

pub async fn get_types() -> impl Responder {
    let mut supported_channels: Vec<Channel> = vec![];
    supported_channels.extend(TotalEnergyConsumption::variants().iter().map(|c| c.into()));
    supported_channels.extend(HeatingDemand::variants().iter().map(|c| c.into()));

    let rows: Vec<TypeAndItem> = supported_channels
        .iter()
        .map(|c| TypeAndItem {
            type_: c.type_name().to_string(),
            item: c.item_name().to_string(),
        })
        .collect();

    csv_response(&rows)
}

pub async fn get_items(path: web::Path<String>) -> impl Responder {
    let type_ = path.into_inner();

    let mut supported_channels: Vec<Channel> = vec![];
    supported_channels.extend(TotalEnergyConsumption::variants().iter().map(|c| c.into()));
    supported_channels.extend(HeatingDemand::variants().iter().map(|c| c.into()));

    let items = supported_channels.iter().filter_map(|c| {
        if type_ == c.type_name() {
            Some(c.item_name().to_string())
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
    let channel = Channel::from_type_and_item(&path.0, &path.1)
        .ok_or_else(|| GrafanaApiError::ChannelNotFound(path.0.to_string(), path.1.to_string()))?;

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
    T: Estimatable + Clone + Into<Channel> + TypedItem,
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

    let rows: Vec<Row> = dps
        .iter()
        .map(|dp| Row {
            timestamp: dp.timestamp,
            type_: item.type_name().to_string(),
            item: item.item_name().to_string(),
            value: *dp.value.as_ref(),
        })
        .collect();

    Ok(rows)
}
