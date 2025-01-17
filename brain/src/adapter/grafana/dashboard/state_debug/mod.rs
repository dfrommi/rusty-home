use std::{cmp::Ordering, sync::Arc};

use actix_web::{
    web::{self, Query},
    Responder,
};
use api::state::{Channel, HeatingDemand, RelativeHumidity, Temperature, TotalEnergyConsumption};
use support::InternalId;
use support::{time::DateTime, DataPoint, ExternalId};

use crate::{
    adapter::grafana::{support::csv_response, GrafanaApiError},
    home::state::DewPoint,
    port::TimeSeriesAccess,
    support::timeseries::interpolate::Estimatable,
};

use super::TimeRangeWithIntervalQuery;

pub fn routes<T>(api: Arc<T>) -> actix_web::Scope
where
    T: TimeSeriesAccess<TotalEnergyConsumption>
        + TimeSeriesAccess<HeatingDemand>
        + TimeSeriesAccess<Temperature>
        + TimeSeriesAccess<RelativeHumidity>
        + 'static,
{
    web::scope("/state")
        .route("", web::get().to(get_types))
        .route("/{type}", web::get().to(get_items))
        .route("/{type}/{item}", web::get().to(state_ts::<T>))
        .app_data(web::Data::from(api))
}

#[derive(serde::Serialize)]
struct Row {
    timestamp: DateTime,
    #[serde(rename = "type")]
    type_: String,
    item: String,
    value: f64,
}

fn supported_channels() -> Vec<ExternalId> {
    let mut supported_channels: Vec<ExternalId> = vec![];
    supported_channels.extend(TotalEnergyConsumption::variants().iter().map(|c| c.into()));
    supported_channels.extend(HeatingDemand::variants().iter().map(|c| c.into()));
    supported_channels.extend(Temperature::variants().iter().map(|c| c.into()));
    supported_channels.extend(RelativeHumidity::variants().iter().map(|c| c.into()));
    supported_channels.extend(DewPoint::variants().iter().map(|c| c.into()));

    supported_channels
}

async fn get_types() -> impl Responder {
    csv_response(supported_channels())
}

async fn get_items(path: web::Path<String>) -> impl Responder {
    let type_ = path.into_inner();

    let supported_channels = supported_channels();
    let items = supported_channels.iter().filter_map(|c| {
        if type_ == c.type_ {
            Some(c.name.to_owned())
        } else {
            None
        }
    });

    csv_response(items.collect::<Vec<_>>())
}

async fn state_ts<T>(
    api: web::Data<T>,
    path: web::Path<(String, String)>,
    time_range: Query<TimeRangeWithIntervalQuery>,
) -> Result<impl Responder, GrafanaApiError>
where
    T: TimeSeriesAccess<TotalEnergyConsumption>
        + TimeSeriesAccess<HeatingDemand>
        + TimeSeriesAccess<Temperature>
        + TimeSeriesAccess<RelativeHumidity>,
{
    let external_id = ExternalId::new(path.0.as_str(), path.1.as_str());
    let channel = Channel::try_from(&external_id)
        .map_err(|_| GrafanaApiError::ChannelNotFound(external_id.clone()))?;

    let mut rows = match channel {
        Channel::Temperature(item) => get_rows(item, api.as_ref(), &time_range).await?,
        Channel::RelativeHumidity(item) => get_rows(item, api.as_ref(), &time_range).await?,
        Channel::TotalEnergyConsumption(item) => get_rows(item, api.as_ref(), &time_range).await?,
        Channel::HeatingDemand(item) => get_rows(item, api.as_ref(), &time_range).await?,
        _ => {
            return Err(GrafanaApiError::ChannelUnsupported(external_id));
        }
    };

    rows.sort_by(|a, b| b.value.partial_cmp(&a.value).unwrap_or(Ordering::Equal));

    csv_response(&rows).map_err(|e| {
        tracing::error!("Error serializing response: {:?}", e);
        e
    })
}

async fn get_rows<T>(
    item: T,
    api: &impl TimeSeriesAccess<T>,
    time_range: &TimeRangeWithIntervalQuery,
) -> Result<Vec<Row>, GrafanaApiError>
where
    T: Estimatable + Clone + Into<InternalId>,
    T::Type: AsRef<f64>,
{
    let ts = api
        .series(item.clone(), time_range.range())
        .await
        .map_err(GrafanaApiError::DataAccessError)?;

    let dps: Vec<DataPoint<<T>::Type>> = time_range
        .iter()
        .filter_map(|t| ts.at(t))
        .collect::<Vec<_>>();

    let int_id: InternalId = item.into();

    let rows: Vec<Row> = dps
        .iter()
        .map(|dp| Row {
            timestamp: dp.timestamp,
            type_: int_id.type_.to_string(),
            item: int_id.name.to_string(),
            value: *dp.value.as_ref(),
        })
        .collect();

    Ok(rows)
}
