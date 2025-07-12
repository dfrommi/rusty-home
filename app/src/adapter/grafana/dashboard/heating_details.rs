use std::sync::Arc;

use crate::core::HomeApi;
use crate::core::time::DateTime;
use crate::home::state::{HeatingDemand, SetPoint, Temperature};
use actix_web::web::{self, Path, Query};

use crate::{
    adapter::grafana::{GrafanaApiError, GrafanaResponse, support::csv_response},
    home::state::Opened,
    port::TimeSeriesAccess,
};

use super::{Room, TimeRangeQuery, TimeRangeWithIntervalQuery};

pub fn routes(api: Arc<HomeApi>) -> actix_web::Scope
where
    Temperature: TimeSeriesAccess<Temperature>,
    SetPoint: TimeSeriesAccess<SetPoint>,
    Opened: TimeSeriesAccess<Opened>,
    HeatingDemand: TimeSeriesAccess<HeatingDemand>,
{
    web::scope("/heating_details/{room}")
        .route("/temperature", web::get().to(temperature_series))
        .route("/temperature/stats", web::get().to(temperature_stats))
        .route("/environment", web::get().to(environment_series))
        .route("/environment/stats", web::get().to(environment_stats))
        .app_data(web::Data::from(api))
}

#[derive(serde::Serialize)]
struct Row {
    timestamp: DateTime,
    channel: &'static str,
    value: f64,
}

async fn temperature_series(
    api: web::Data<HomeApi>,
    path: Path<Room>,
    query: Query<TimeRangeWithIntervalQuery>,
) -> GrafanaResponse
where
    Temperature: TimeSeriesAccess<Temperature>,
    SetPoint: TimeSeriesAccess<SetPoint>,
{
    let room = path.into_inner();

    let inside_temp = room.inside_temperature();
    let set_point = room.set_point();
    let (ts_outside, ts_inside, ts_set_point) = tokio::try_join!(
        Temperature::Outside.series(query.range(), api.as_ref()),
        inside_temp.series(query.range(), api.as_ref()),
        set_point.series(query.range(), api.as_ref()),
    )
    .map_err(GrafanaApiError::DataAccessError)?;

    let mut rows: Vec<Row> = vec![];
    for dt in query.iter() {
        if let Some(dp) = ts_outside.at(dt) {
            rows.push(Row {
                channel: "outside_temperature",
                timestamp: dp.timestamp,
                value: dp.value.0,
            })
        }
        if let Some(dp) = ts_inside.at(dt) {
            rows.push(Row {
                channel: "inside_temperature",
                timestamp: dp.timestamp,
                value: dp.value.0,
            })
        }

        if let Some(dp) = ts_set_point.at(dt) {
            rows.push(Row {
                channel: "target_temperature",
                timestamp: dp.timestamp,
                value: dp.value.0,
            })
        }
    }

    csv_response(&rows)
}

async fn environment_series(
    api: web::Data<HomeApi>,
    path: Path<Room>,
    query: Query<TimeRangeWithIntervalQuery>,
) -> GrafanaResponse
where
    Opened: TimeSeriesAccess<Opened>,
    HeatingDemand: TimeSeriesAccess<HeatingDemand>,
{
    let room = path.into_inner();

    let window = room.window();
    let heating_demand = room.heating_demand();
    let (ts_opened, ts_heating) = tokio::try_join!(
        window.series(query.range(), api.as_ref()),
        heating_demand.series(query.range(), api.as_ref()),
    )
    .map_err(GrafanaApiError::DataAccessError)?;

    let mut rows: Vec<Row> = vec![];
    for dt in query.iter() {
        if let Some(dp) = ts_opened.at(dt) {
            rows.push(Row {
                channel: "window_opened",
                timestamp: dp.timestamp,
                value: if dp.value { 100.0 } else { 0.0 },
            })
        }
        if let Some(dp) = ts_heating.at(dt) {
            rows.push(Row {
                channel: "heating_demand",
                timestamp: dp.timestamp,
                value: dp.value.0,
            })
        }
    }

    csv_response(&rows)
}

async fn temperature_stats(
    api: web::Data<HomeApi>,
    room: Path<Room>,
    query: Query<TimeRangeQuery>,
) -> GrafanaResponse
where
    Temperature: TimeSeriesAccess<Temperature>,
    SetPoint: TimeSeriesAccess<SetPoint>,
{
    #[derive(serde::Serialize)]
    struct Row {
        channel: &'static str,
        mean: f64,
    }

    let inside_temp = room.inside_temperature();
    let set_point = room.set_point();
    let (ts_outside, ts_inside, ts_set_point) = tokio::try_join!(
        Temperature::Outside.series(query.range(), api.as_ref()),
        inside_temp.series(query.range(), api.as_ref()),
        set_point.series(query.range(), api.as_ref()),
    )
    .map_err(GrafanaApiError::DataAccessError)?;

    let rows: Vec<Row> = vec![
        Row {
            channel: "outside_temperature",
            mean: ts_outside.mean().0,
        },
        Row {
            channel: "inside_temperature",
            mean: ts_inside.mean().0,
        },
        Row {
            channel: "target_temperature",
            mean: ts_set_point.mean().0,
        },
    ];

    csv_response(&rows)
}

async fn environment_stats(
    api: web::Data<HomeApi>,
    room: Path<Room>,
    query: Query<TimeRangeQuery>,
) -> GrafanaResponse
where
    HeatingDemand: TimeSeriesAccess<HeatingDemand>,
{
    #[derive(serde::Serialize)]
    struct Row {
        channel: &'static str,
        sum: f64,
    }

    let heating_demand = room.heating_demand();
    let ts_heating = heating_demand
        .series(query.range(), api.as_ref())
        .await
        .map_err(GrafanaApiError::DataAccessError)?;

    let rows: Vec<Row> = vec![Row {
        channel: "heating_demand",
        sum: ts_heating.area_in_type_hours() * room.heating_factor(),
    }];

    csv_response(&rows)
}
