use std::sync::Arc;

use actix_web::{
    Responder,
    web::{self, Query},
};
use api::state::{HeatingDemand, Temperature};
use support::{
    time::{DateTime, DateTimeRange, Duration},
    unit::Percent,
};

use crate::{
    adapter::grafana::{GrafanaApiError, support::csv_response},
    core::timeseries::TimeSeries,
    port::TimeSeriesAccess,
};

use super::Room;

pub fn routes<T>(api: Arc<T>) -> actix_web::Scope
where
    T: TimeSeriesAccess<HeatingDemand> + TimeSeriesAccess<Temperature> + 'static,
{
    web::scope("/energy_iq")
        .route(
            "/consumption/series",
            web::get().to(heating_series_aggregated_sum::<T>),
        )
        .route(
            "/temperature/delta",
            web::get().to(outside_temperature_series::<T>),
        )
        .app_data(web::Data::from(api))
}

#[derive(Clone, Debug, serde::Deserialize)]
struct QueryTimeRange {
    from: DateTime,
    to: DateTime,
    offset: Option<Duration>,
    #[serde(deserialize_with = "super::empty_string_as_none")]
    room: Option<Room>,
}

impl QueryTimeRange {
    fn offset(&self) -> Duration {
        self.offset.clone().unwrap_or_else(Duration::zero)
    }

    fn ts_range_no_offset(&self) -> DateTimeRange {
        DateTimeRange::new(self.from, self.to).non_future()
    }

    fn ts_range(&self) -> DateTimeRange {
        DateTimeRange::new(self.from - self.offset(), self.to - self.offset()).non_future()
    }
}

#[derive(serde::Serialize)]
struct Row {
    timestamp: DateTime,
    value: f64,
}

async fn heating_series_aggregated_sum<T>(
    api: web::Data<T>,
    query: Query<QueryTimeRange>,
) -> Result<impl Responder, GrafanaApiError>
where
    T: TimeSeriesAccess<HeatingDemand>,
{
    let rooms = match &query.room {
        Some(room) => vec![room.clone()],
        None => Room::variants().to_vec(),
    };

    let ts = combined_series(api.as_ref(), &rooms, query.ts_range())
        .await
        .map_err(GrafanaApiError::DataAccessError)?;

    let mut previous = 0.0;
    let mut rows: Vec<Row> = vec![];

    for dp in ts.area_series_in_unit_hours().iter() {
        let value = previous + dp.value.0;

        rows.push(Row {
            timestamp: dp.timestamp + query.offset(),
            value,
        });

        previous = value;
    }

    csv_response(&rows)
}

async fn outside_temperature_series<T>(
    api: web::Data<T>,
    query: Query<QueryTimeRange>,
) -> Result<impl Responder, GrafanaApiError>
where
    T: TimeSeriesAccess<Temperature>,
{
    let (ts, ts_ref) = tokio::try_join!(
        api.series(Temperature::Outside, query.ts_range_no_offset()),
        api.series(Temperature::Outside, query.ts_range()) //TODO skip future DPs
    )
    .map_err(GrafanaApiError::DataAccessError)?;

    let rows: Vec<Row> = ts
        .inner()
        .iter()
        .map(|dp| {
            let ref_value = ts_ref
                .at(dp.timestamp - query.offset())
                .map(|dp| dp.value.0);
            let value = match ref_value {
                Some(v) => dp.value.0 - v,
                None => 0.0,
            };

            Row {
                value,
                timestamp: dp.timestamp,
            }
        })
        .collect();

    Ok(csv_response(&rows))
}

async fn combined_series(
    api: &impl TimeSeriesAccess<HeatingDemand>,
    rooms: &[Room],
    time_range: DateTimeRange,
) -> anyhow::Result<TimeSeries<HeatingDemand>> {
    let rooms_ts = rooms.iter().map(|room| async {
        match api.series(room.heating_demand(), time_range.clone()).await {
            Ok(ts) => Ok((room.clone(), ts)),
            Err(e) => Err(e),
        }
    });

    let rooms_ts = futures::future::join_all(rooms_ts)
        .await
        .into_iter()
        .collect::<Result<Vec<_>, _>>()?;

    let mut mapped_ts = rooms_ts
        .into_iter()
        .map(|(room, ts)| {
            let factor = room.heating_factor();
            let context = ts.context();
            ts.map(context, |dp| {
                let value: f64 = dp.value.0;
                Percent(value * factor)
            })
        })
        .collect::<Vec<_>>();

    let mut result = mapped_ts.remove(0);
    for ts in mapped_ts {
        result = TimeSeries::combined(&result, &ts, HeatingDemand::LivingRoom, |a, b| {
            Percent(a.0 + b.0)
        })?;
    }

    Ok(result)
}
