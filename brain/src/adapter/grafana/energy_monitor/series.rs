use actix_web::{
    web::{self, Query},
    Responder,
};
use api::state::{HeatingDemand, Temperature};
use support::{
    time::{DateTime, DateTimeRange, Duration},
    unit::Percent,
};

use crate::{
    adapter::grafana::{csv_response, energy_monitor::heating_factor, GrafanaApiError},
    port::TimeSeriesAccess,
    support::timeseries::TimeSeries,
};

#[derive(Clone, Debug, serde::Deserialize)]
pub struct QueryTimeRange {
    from: DateTime,
    to: DateTime,
    offset: Option<Duration>,
    room: String,
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

pub async fn heating_series_aggregated_sum<T>(
    api: web::Data<T>,
    query: Query<QueryTimeRange>,
) -> Result<impl Responder, GrafanaApiError>
where
    T: TimeSeriesAccess<HeatingDemand>,
{
    let items = if query.room == "_all_" {
        HeatingDemand::variants()
    } else {
        match HeatingDemand::from_item_name(&query.room) {
            Some(item) => &[item],
            None => {
                return Err(GrafanaApiError::ChannelNotFound(
                    HeatingDemand::TYPE_NAME.to_string(),
                    query.room.to_string(),
                ))
            }
        }
    };

    let ts = combined_series(api.as_ref(), items, query.ts_range())
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

    Ok(csv_response(&rows))
}

pub async fn outside_temperature_series<T>(
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
    items: &[HeatingDemand],
    time_range: DateTimeRange,
) -> anyhow::Result<TimeSeries<HeatingDemand>> {
    let items_ts = items.iter().map(|item| async {
        match api.series(item.clone(), time_range.clone()).await {
            Ok(ts) => Ok((item.clone(), ts)),
            Err(e) => Err(e),
        }
    });

    let items_ts = futures::future::join_all(items_ts)
        .await
        .into_iter()
        .collect::<Result<Vec<_>, _>>()?;

    let mut mapped_ts = items_ts
        .into_iter()
        .map(|(item, ts)| {
            let factor = heating_factor(&item);
            ts.map(|dp| {
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
