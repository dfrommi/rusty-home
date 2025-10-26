use std::cmp::Ordering;

use crate::core::HomeApi;
use crate::core::time::DateTimeRange;
use crate::home::Thermostat;
use crate::home::state::{HeatingDemand, HomeStateValueType, TotalEnergyConsumption};
use actix_web::{
    HttpResponse, Responder,
    http::header,
    web::{self, Query},
};

use crate::{
    adapter::grafana::{
        DashboardDisplay,
        dashboard::{EURO_PER_KWH, TimeRangeQuery},
    },
    core::timeseries::{TimeSeries, interpolate::Estimatable},
    port::TimeSeriesAccess,
};

pub async fn total_power(api: web::Data<HomeApi>, time_range: Query<TimeRangeQuery>) -> impl Responder {
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

pub async fn total_heating(api: web::Data<HomeApi>, time_range: Query<TimeRangeQuery>) -> impl Responder {
    total_values_response(api.as_ref(), HeatingDemand::variants(), time_range.range(), |item, ts| {
        let value = ts.area_in_type_hours();
        (value, value * thermostat_of(item).heating_factor())
    })
    .await
}

async fn total_values_response<T, V: Fn(&T, TimeSeries<T>) -> (f64, f64)>(
    api: &HomeApi,
    items: &[T],
    time_range: DateTimeRange,
    value_mapper: V,
) -> impl Responder + use<T, V>
where
    T: HomeStateValueType
        + DashboardDisplay
        + Estimatable
        + Clone
        + std::fmt::Debug
        + Into<crate::home::state::PersistentHomeState>
        + TimeSeriesAccess<T>,
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
        let result = item.clone().series(time_range.clone(), api).await;

        if let Err(e) = result {
            return HttpResponse::InternalServerError().body(format!("Error retrieving data: {e}"));
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

fn thermostat_of(item: &HeatingDemand) -> Thermostat {
    match item {
        HeatingDemand::LivingRoomBig => Thermostat::LivingRoomBig,
        HeatingDemand::LivingRoomSmall => Thermostat::LivingRoomSmall,
        HeatingDemand::Bedroom => Thermostat::Bedroom,
        HeatingDemand::RoomOfRequirements => Thermostat::RoomOfRequirements,
        HeatingDemand::Kitchen => Thermostat::Kitchen,
        HeatingDemand::Bathroom => Thermostat::Bathroom,
    }
}
