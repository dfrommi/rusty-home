use std::cmp::Ordering;

use actix_web::{
    http::header,
    web::{self, Query},
    HttpResponse, Responder,
};
use api::state::{ChannelTypeInfo, HeatingDemand, TotalEnergyConsumption};
use strum::VariantArray;
use support::time::DateTimeRange;

use crate::{
    adapter::grafana::{DashboardDisplay, QueryTimeRange},
    port::TimeSeriesAccess,
    support::timeseries::{interpolate::Estimatable, TimeSeries},
};

const EURO_PER_KWH: f64 = 0.349;

fn heating_factor(item: &HeatingDemand) -> f64 {
    match item {
        HeatingDemand::LivingRoom => 1.728 + 0.501,
        HeatingDemand::Bedroom => 1.401,
        HeatingDemand::RoomOfRequirements => 1.193,
        HeatingDemand::Kitchen => 1.485,
        HeatingDemand::Bathroom => 0.496,
    }
}

pub async fn total_power<T>(api: web::Data<T>, time_range: Query<QueryTimeRange>) -> impl Responder
where
    T: TimeSeriesAccess<TotalEnergyConsumption>,
{
    let time_range: DateTimeRange = time_range.range();

    total_values_response(
        api.as_ref(),
        TotalEnergyConsumption::VARIANTS,
        time_range.clone(),
        move |_, ts| {
            let value = ts.last().value.0 - ts.first().value.0;
            (value, value * EURO_PER_KWH)
        },
    )
    .await
}

pub async fn total_heating<T>(
    api: web::Data<T>,
    time_range: Query<QueryTimeRange>,
) -> impl Responder
where
    T: TimeSeriesAccess<HeatingDemand>,
{
    total_values_response(
        api.as_ref(),
        HeatingDemand::VARIANTS,
        time_range.range(),
        |item, ts| {
            let value = ts.area_in_type_hours();
            (value, value * heating_factor(item))
        },
    )
    .await
}

async fn total_values_response<T>(
    api: &impl TimeSeriesAccess<T>,
    items: &[T],
    time_range: DateTimeRange,
    value_mapper: impl Fn(&T, TimeSeries<T>) -> (f64, f64),
) -> impl Responder
where
    T: ChannelTypeInfo + DashboardDisplay + Estimatable + Clone,
    T::ValueType: PartialOrd + AsRef<f64>,
{
    struct Row {
        name: String,
        value: f64,
        price: f64,
    }

    let time_range = time_range.non_future();

    let mut rows: Vec<Row> = vec![];

    for item in items {
        let result = api.series(item.clone(), time_range.clone()).await;

        if let Err(e) = result {
            return HttpResponse::InternalServerError()
                .body(format!("Error retrieving data: {}", e));
        }
        let result = result.unwrap();

        let (value, price) = value_mapper(item, result);

        rows.push(Row {
            name: DashboardDisplay::display(item).to_string(),
            value,
            price,
        });
    }

    rows.sort_by(|a, b| b.value.partial_cmp(&a.value).unwrap_or(Ordering::Equal));
    let mut csv = "name,raw,value\n".to_string();
    for row in rows {
        csv.push_str(&format!("{},{},{}\n", row.name, row.value, row.price));
    }

    HttpResponse::Ok()
        .append_header(header::ContentType(mime::TEXT_CSV))
        .body(csv)
}
