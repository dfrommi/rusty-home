use std::sync::Arc;

use crate::core::time::DateTime;
use crate::home::state::{HeatingDemand, SetPoint, Temperature};
use actix_web::web::{self, Path, Query};

use crate::{
    adapter::grafana::{GrafanaApiError, GrafanaResponse, support::csv_response},
    home::state::Opened,
    port::TimeSeriesAccess,
};

use super::{Room, TimeRangeQuery, TimeRangeWithIntervalQuery};

pub fn routes<T>(api: Arc<T>) -> actix_web::Scope
where
    T: TimeSeriesAccess<Temperature>
        + TimeSeriesAccess<SetPoint>
        + TimeSeriesAccess<Opened>
        + TimeSeriesAccess<HeatingDemand>
        + 'static,
{
    web::scope("/heating_details/{room}")
        .route("/temperature", web::get().to(temperature_series::<T>))
        .route("/temperature/stats", web::get().to(temperature_stats::<T>))
        .route("/environment", web::get().to(environment_series::<T>))
        .route("/environment/stats", web::get().to(environment_stats::<T>))
        .app_data(web::Data::from(api))
}

#[derive(serde::Serialize)]
struct Row {
    timestamp: DateTime,
    channel: &'static str,
    value: f64,
}

async fn temperature_series<T>(
    api: web::Data<T>,
    path: Path<Room>,
    query: Query<TimeRangeWithIntervalQuery>,
) -> GrafanaResponse
where
    T: TimeSeriesAccess<Temperature> + TimeSeriesAccess<SetPoint>,
{
    let room = path.into_inner();

    let (ts_outside, ts_inside, ts_set_point) = tokio::try_join!(
        api.series(Temperature::Outside, query.range()),
        api.series(room.inside_temperature(), query.range()),
        api.series(room.set_point(), query.range()),
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

async fn environment_series<T>(
    api: web::Data<T>,
    path: Path<Room>,
    query: Query<TimeRangeWithIntervalQuery>,
) -> GrafanaResponse
where
    T: TimeSeriesAccess<Opened> + TimeSeriesAccess<HeatingDemand>,
{
    let room = path.into_inner();

    let (ts_opened, ts_heating) = tokio::try_join!(
        api.series(room.window(), query.range()),
        api.series(room.heating_demand(), query.range()),
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

async fn temperature_stats<T>(api: web::Data<T>, room: Path<Room>, query: Query<TimeRangeQuery>) -> GrafanaResponse
where
    T: TimeSeriesAccess<Temperature> + TimeSeriesAccess<SetPoint>,
{
    #[derive(serde::Serialize)]
    struct Row {
        channel: &'static str,
        mean: f64,
    }

    let (ts_outside, ts_inside, ts_set_point) = tokio::try_join!(
        api.series(Temperature::Outside, query.range()),
        api.series(room.inside_temperature(), query.range()),
        api.series(room.set_point(), query.range()),
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

async fn environment_stats<T>(api: web::Data<T>, room: Path<Room>, query: Query<TimeRangeQuery>) -> GrafanaResponse
where
    T: TimeSeriesAccess<HeatingDemand>,
{
    #[derive(serde::Serialize)]
    struct Row {
        channel: &'static str,
        sum: f64,
    }

    let ts_heating = api
        .series(room.heating_demand(), query.range())
        .await
        .map_err(GrafanaApiError::DataAccessError)?;

    let rows: Vec<Row> = vec![Row {
        channel: "heating_demand",
        sum: ts_heating.area_in_type_hours() * room.heating_factor(),
    }];

    csv_response(&rows)
}
