use std::cmp::Ordering;

use actix_web::{
    web::{self, Query},
    Responder,
};
use api::state::{HeatingDemand, RelativeHumidity, Temperature, TotalEnergyConsumption};
use support::TypedItem;
use support::{time::DateTime, DataPoint};

use crate::{
    adapter::grafana::csv_response, home::state::DewPoint, port::TimeSeriesAccess,
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

fn supported_channels() -> Vec<TypeAndItem> {
    let mut supported_channels: Vec<TypeAndItem> = vec![];
    supported_channels.extend(TotalEnergyConsumption::variants().iter().map(|c| c.into()));
    supported_channels.extend(HeatingDemand::variants().iter().map(|c| c.into()));
    supported_channels.extend(Temperature::variants().iter().map(|c| c.into()));
    supported_channels.extend(RelativeHumidity::variants().iter().map(|c| c.into()));
    supported_channels.extend(DewPoint::variants().iter().map(|c| c.into()));

    supported_channels
}

impl<T: TypedItem> From<&T> for TypeAndItem {
    fn from(val: &T) -> Self {
        TypeAndItem {
            type_: val.type_name().to_string(),
            item: val.item_name().to_string(),
        }
    }
}

pub async fn get_types() -> impl Responder {
    csv_response(&supported_channels())
}

pub async fn get_items(path: web::Path<String>) -> impl Responder {
    let type_ = path.into_inner();

    let supported_channels = supported_channels();
    let items = supported_channels.iter().filter_map(|c| {
        if type_ == c.type_ {
            Some(c.item.to_owned())
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
    T: TimeSeriesAccess<TotalEnergyConsumption>
        + TimeSeriesAccess<HeatingDemand>
        + TimeSeriesAccess<Temperature>
        + TimeSeriesAccess<RelativeHumidity>,
{
    macro_rules! from_type_and_item {
        ($type:ident) => {
            get_rows(
                $type::from_item_name(&path.1).ok_or(GrafanaApiError::ChannelNotFound(
                    path.0.to_string(),
                    path.1.to_string(),
                ))?,
                api.as_ref(),
                &time_range,
            )
            .await
        };
    }

    let mut rows = match path.0.as_str() {
        TotalEnergyConsumption::TYPE_NAME => from_type_and_item!(TotalEnergyConsumption),
        HeatingDemand::TYPE_NAME => from_type_and_item!(HeatingDemand),
        Temperature::TYPE_NAME => from_type_and_item!(Temperature),
        RelativeHumidity::TYPE_NAME => from_type_and_item!(RelativeHumidity),
        DewPoint::TYPE_NAME => from_type_and_item!(DewPoint),
        _ => {
            return Err(GrafanaApiError::ChannelUnsupported(
                path.0.to_string(),
                path.1.to_string(),
            ))
        }
    }?;

    rows.sort_by(|a, b| b.value.partial_cmp(&a.value).unwrap_or(Ordering::Equal));

    csv_response(&rows).map_err(|e| {
        tracing::error!("Error serializing response: {:?}", e);
        e
    })
}

async fn get_rows<T>(
    item: T,
    api: &impl TimeSeriesAccess<T>,
    time_range: &QueryTimeRange,
) -> Result<Vec<Row>, GrafanaApiError>
where
    T: Estimatable + Clone + TypedItem,
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
