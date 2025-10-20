use std::sync::Arc;

use crate::core::HomeApi;
use crate::core::time::DateTime;
use crate::home::HeatingZone;
use crate::home::state::{HeatingDemand, HeatingMode, ScheduledHeatingMode, SetPoint, Temperature};
use actix_web::web::{self, Path, Query};

use crate::{
    adapter::grafana::{GrafanaApiError, GrafanaResponse, support::csv_response},
    home::state::OpenedArea,
    port::TimeSeriesAccess,
};

use super::{TimeRangeQuery, TimeRangeWithIntervalQuery};

pub fn routes(api: Arc<HomeApi>) -> actix_web::Scope
where
    Temperature: TimeSeriesAccess<Temperature>,
    SetPoint: TimeSeriesAccess<SetPoint>,
    OpenedArea: TimeSeriesAccess<OpenedArea>,
    HeatingDemand: TimeSeriesAccess<HeatingDemand>,
{
    web::scope("/heating_details/{room}")
        .route("/temperature", web::get().to(temperature_series))
        .route("/temperature/stats", web::get().to(temperature_stats))
        .route("/environment", web::get().to(environment_series))
        .route("/environment/stats", web::get().to(environment_stats))
        .route("/schedule", web::get().to(schedule_series))
        .app_data(web::Data::from(api))
}

#[derive(serde::Serialize)]
struct Row {
    timestamp: DateTime,
    channel: String,
    value: f64,
}

async fn temperature_series(
    api: web::Data<HomeApi>,
    path: Path<HeatingZone>,
    query: Query<TimeRangeWithIntervalQuery>,
) -> GrafanaResponse
where
    Temperature: TimeSeriesAccess<Temperature>,
    SetPoint: TimeSeriesAccess<SetPoint>,
{
    let heating_zone = path.into_inner();
    let inside_temp = heating_zone.inside_temperature();

    let (ts_outside, ts_inside) = tokio::try_join!(
        Temperature::Outside.series(query.range(), api.as_ref()),
        inside_temp.series(query.range(), api.as_ref()),
    )
    .map_err(GrafanaApiError::DataAccessError)?;

    let mut ts_set_points = vec![];
    for thermostat in heating_zone.thermostats() {
        let ts_set_point = thermostat
            .set_point()
            .series(query.range(), api.as_ref())
            .await
            .map_err(GrafanaApiError::DataAccessError)?;
        ts_set_points.push(ts_set_point);
    }

    let mut rows: Vec<Row> = vec![];
    for dt in query.iter() {
        if let Some(dp) = ts_outside.at(dt) {
            rows.push(Row {
                channel: "outside_temperature".to_string(),
                timestamp: dp.timestamp,
                value: dp.value.0,
            })
        }
        if let Some(dp) = ts_inside.at(dt) {
            rows.push(Row {
                channel: "inside_temperature".to_string(),
                timestamp: dp.timestamp,
                value: dp.value.0,
            })
        }

        for (i, ts_set_point) in ts_set_points.iter().enumerate() {
            if let Some(dp) = ts_set_point.at(dt) {
                rows.push(Row {
                    channel: format!("target_temperature_{}", i + 1),
                    timestamp: dp.timestamp,
                    value: dp.value.0,
                })
            }
        }
    }

    csv_response(&rows)
}

async fn environment_series(
    api: web::Data<HomeApi>,
    path: Path<HeatingZone>,
    query: Query<TimeRangeWithIntervalQuery>,
) -> GrafanaResponse
where
    OpenedArea: TimeSeriesAccess<OpenedArea>,
    HeatingDemand: TimeSeriesAccess<HeatingDemand>,
{
    let heating_zone = path.into_inner();
    let window = heating_zone.window();

    let ts_opened = window
        .series(query.range(), api.as_ref())
        .await
        .map_err(GrafanaApiError::DataAccessError)?;

    let mut ts_heating_demands = vec![];
    for thermostat in heating_zone.thermostats() {
        let ts_heating_demand = thermostat
            .heating_demand()
            .series(query.range(), api.as_ref())
            .await
            .map_err(GrafanaApiError::DataAccessError)?;
        ts_heating_demands.push(ts_heating_demand);
    }

    let mut rows: Vec<Row> = vec![];
    for dt in query.iter() {
        if let Some(dp) = ts_opened.at(dt) {
            rows.push(Row {
                channel: "window_opened".to_string(),
                timestamp: dp.timestamp,
                value: if dp.value { 100.0 } else { 0.0 },
            })
        }

        for (i, ts_heating) in ts_heating_demands.iter().enumerate() {
            let channel_name = format!("heating_demand_{}", i + 1);
            if let Some(dp) = ts_heating.at(dt) {
                rows.push(Row {
                    channel: channel_name.to_string(),
                    timestamp: dp.timestamp,
                    value: dp.value.0,
                })
            }
        }
    }

    csv_response(&rows)
}

async fn schedule_series(
    api: web::Data<HomeApi>,
    path: Path<HeatingZone>,
    query: Query<TimeRangeWithIntervalQuery>,
) -> GrafanaResponse {
    let heating_zone = path.into_inner();

    let schedule = match heating_zone {
        HeatingZone::LivingRoom => ScheduledHeatingMode::LivingRoom,
        HeatingZone::Bedroom => ScheduledHeatingMode::Bedroom,
        HeatingZone::Kitchen => ScheduledHeatingMode::Kitchen,
        HeatingZone::RoomOfRequirements => ScheduledHeatingMode::RoomOfRequirements,
        HeatingZone::Bathroom => ScheduledHeatingMode::Bathroom,
    };

    let ts = schedule
        .series(query.range(), api.as_ref())
        .await
        .map_err(GrafanaApiError::DataAccessError)?;

    let mut rows: Vec<Row> = vec![];

    for dt in query.iter() {
        let Some(dp) = ts.at(dt) else {
            continue;
        };

        for mode in HeatingMode::variants() {
            let v = if dp.value == *mode { 100.0 } else { 0.0 };

            rows.push(Row {
                channel: mode.ext_id().variant_name().to_string(),
                timestamp: dp.timestamp,
                value: v,
            })
        }
    }

    csv_response(&rows)
}
async fn temperature_stats(
    api: web::Data<HomeApi>,
    path: Path<HeatingZone>,
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

    let heating_zone = path.into_inner();
    let inside_temp = heating_zone.inside_temperature();
    let (ts_outside, ts_inside) = tokio::try_join!(
        Temperature::Outside.series(query.range(), api.as_ref()),
        inside_temp.series(query.range(), api.as_ref()),
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
    ];

    csv_response(&rows)
}

async fn environment_stats(
    api: web::Data<HomeApi>,
    path: Path<HeatingZone>,
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

    let mut total_demand_scaled = 0.0;

    let zone = path.into_inner();
    for thermostat in zone.thermostats() {
        let ts_heating = thermostat
            .heating_demand()
            .series(query.range(), api.as_ref())
            .await
            .map_err(GrafanaApiError::DataAccessError)?;

        total_demand_scaled = total_demand_scaled + ts_heating.area_in_type_hours() * thermostat.heating_factor();
    }

    let rows: Vec<Row> = vec![Row {
        channel: "heating_demand",
        sum: total_demand_scaled,
    }];

    csv_response(&rows)
}
