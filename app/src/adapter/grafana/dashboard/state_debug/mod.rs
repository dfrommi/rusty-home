use std::{cmp::Ordering, sync::Arc};

use crate::core::HomeApi;
use crate::core::id::ExternalId;
use crate::core::time::DateTime;
use crate::home::state::HomeState;
use crate::{
    core::timeseries::DataPoint,
    home::state::{HeatingDemand, PersistentHomeState, RelativeHumidity, Temperature, TotalEnergyConsumption},
};
use actix_web::{
    Responder,
    web::{self, Query},
};

use crate::{
    adapter::grafana::{GrafanaApiError, support::csv_response},
    core::timeseries::interpolate::Estimatable,
    home::state::DewPoint,
    port::TimeSeriesAccess,
};

use super::TimeRangeWithIntervalQuery;

pub fn routes(api: Arc<HomeApi>) -> actix_web::Scope
where
    TotalEnergyConsumption: TimeSeriesAccess<TotalEnergyConsumption>,
    HeatingDemand: TimeSeriesAccess<HeatingDemand>,
    Temperature: TimeSeriesAccess<Temperature>,
    RelativeHumidity: TimeSeriesAccess<RelativeHumidity>,
{
    web::scope("/state")
        .route("", web::get().to(get_types))
        .route("/{type}", web::get().to(get_items))
        .route("/{type}/{item}", web::get().to(state_ts))
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

fn supported_channels() -> Vec<&'static ExternalId> {
    let mut supported_channels: Vec<&'static ExternalId> = vec![];
    supported_channels.extend(TotalEnergyConsumption::variants().iter().map(|c| c.ext_id()));
    supported_channels.extend(HeatingDemand::variants().iter().map(|c| c.ext_id()));
    supported_channels.extend(Temperature::variants().iter().map(|c| c.ext_id()));
    supported_channels.extend(RelativeHumidity::variants().iter().map(|c| c.ext_id()));
    supported_channels.extend(DewPoint::variants().iter().map(|c| c.ext_id()));

    supported_channels
}

async fn get_types() -> impl Responder {
    csv_response(supported_channels())
}

async fn get_items(path: web::Path<String>) -> impl Responder {
    let type_ = path.into_inner();

    let supported_channels = supported_channels();
    let items = supported_channels.iter().filter_map(|c| {
        if type_ == c.ext_type() {
            Some(c.ext_name().to_owned())
        } else {
            None
        }
    });

    csv_response(items.collect::<Vec<_>>())
}

async fn state_ts(
    api: web::Data<HomeApi>,
    path: web::Path<(String, String)>,
    time_range: Query<TimeRangeWithIntervalQuery>,
) -> Result<impl Responder, GrafanaApiError>
where
    TotalEnergyConsumption: TimeSeriesAccess<TotalEnergyConsumption>,
    HeatingDemand: TimeSeriesAccess<HeatingDemand>,
    Temperature: TimeSeriesAccess<Temperature>,
    RelativeHumidity: TimeSeriesAccess<RelativeHumidity>,
{
    let external_id = ExternalId::new(path.0.as_str(), path.1.as_str());
    let channel = PersistentHomeState::try_from(&external_id)
        .map_err(|_| GrafanaApiError::ChannelNotFound(external_id.clone()))?;

    let mut rows = match channel {
        PersistentHomeState::Temperature(item) => get_rows(item, api.as_ref(), &time_range).await?,
        PersistentHomeState::RelativeHumidity(item) => get_rows(item, api.as_ref(), &time_range).await?,
        PersistentHomeState::TotalEnergyConsumption(item) => get_rows(item, api.as_ref(), &time_range).await?,
        PersistentHomeState::HeatingDemand(item) => get_rows(item, api.as_ref(), &time_range).await?,
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
    api: &HomeApi,
    time_range: &TimeRangeWithIntervalQuery,
) -> Result<Vec<Row>, GrafanaApiError>
where
    T: Estimatable + Clone + Into<HomeState> + TimeSeriesAccess<T>,
    T::ValueType: AsRef<f64>,
{
    let ts = item
        .clone()
        .series(time_range.range(), api)
        .await
        .map_err(GrafanaApiError::DataAccessError)?;

    let dps: Vec<DataPoint<<T>::ValueType>> = time_range.iter().filter_map(|t| ts.at(t)).collect::<Vec<_>>();

    let ext_id = item.clone().into().ext_id();

    let rows: Vec<Row> = dps
        .iter()
        .map(|dp| Row {
            timestamp: dp.timestamp,
            type_: ext_id.ext_type().to_string(),
            item: ext_id.ext_name().to_string(),
            value: *dp.value.as_ref(),
        })
        .collect();

    Ok(rows)
}
