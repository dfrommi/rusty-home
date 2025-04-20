use std::cmp::Ordering;

use actix_web::{
    HttpResponse, Responder,
    http::header,
    web::{self, Query},
};
use api::state::{HeatingDemand, TotalEnergyConsumption};
use support::{ValueObject, time::DateTimeRange};

use crate::{
    adapter::grafana::{
        DashboardDisplay,
        dashboard::{EURO_PER_KWH, Room, TimeRangeQuery},
    },
    port::TimeSeriesAccess,
    support::timeseries::{TimeSeries, interpolate::Estimatable},
};

pub async fn total_power<T>(api: web::Data<T>, time_range: Query<TimeRangeQuery>) -> impl Responder
where
    T: TimeSeriesAccess<TotalEnergyConsumption>,
{
    let time_range: DateTimeRange = time_range.range();

    total_values_response(
        api.as_ref(),
        TotalEnergyConsumption::variants(),
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
    time_range: Query<TimeRangeQuery>,
) -> impl Responder
where
    T: TimeSeriesAccess<HeatingDemand>,
{
    total_values_response(
        api.as_ref(),
        HeatingDemand::variants(),
        time_range.range(),
        |item, ts| {
            let value = ts.area_in_type_hours();
            (value, value * room_of(item).heating_factor())
        },
    )
    .await
}

async fn total_values_response<T, U: TimeSeriesAccess<T>, V: Fn(&T, TimeSeries<T>) -> (f64, f64)>(
    api: &U,
    items: &[T],
    time_range: DateTimeRange,
    value_mapper: V,
) -> impl Responder + use<T, U, V>
where
    T: ValueObject + DashboardDisplay + Estimatable + Clone,
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

fn room_of(item: &HeatingDemand) -> Room {
    match item {
        HeatingDemand::LivingRoom => Room::LivingRoom,
        HeatingDemand::Bedroom => Room::Bedroom,
        HeatingDemand::RoomOfRequirements => Room::RoomOfRequirements,
        HeatingDemand::Kitchen => Room::Kitchen,
        HeatingDemand::Bathroom => Room::Bathroom,
    }
}
