use std::cmp::Ordering;

use crate::core::HomeApi;
use crate::core::ValueObject;
use crate::{
    core::timeseries::DataPoint,
    home::state::{CurrentPowerUsage, HeatingDemand},
};
use actix_web::{
    HttpResponse, Responder,
    http::header,
    web::{self},
};

use crate::{adapter::grafana::DashboardDisplay, port::DataPointAccess};

pub async fn current_power(api: web::Data<HomeApi>) -> impl Responder {
    current_values_response(api.as_ref(), CurrentPowerUsage::variants()).await
}

pub async fn current_heating(api: web::Data<HomeApi>) -> impl Responder {
    current_values_response(api.as_ref(), HeatingDemand::variants()).await
}

async fn current_values_response<T>(api: &HomeApi, items: &[T]) -> impl Responder + use<T>
where
    T: ValueObject + DashboardDisplay + Clone + DataPointAccess<T>,
    T::ValueType: PartialOrd + AsRef<f64>,
{
    let values = get_all_states(api, items).await;
    if let Err(e) = values {
        return HttpResponse::InternalServerError().body(format!("Error: {e}"));
    }

    let mut values = values.unwrap();
    values.sort_by(|(_, a), (_, b)| b.value.partial_cmp(&a.value).unwrap_or(Ordering::Equal));

    let mut csv = "name,value\n".to_string();

    for (item, dp) in values {
        let value = dp.value.as_ref();
        let line = format!("{},{}\n", DashboardDisplay::display(&item), value);
        csv.push_str(&line);
    }

    HttpResponse::Ok()
        .append_header(header::ContentType(mime::TEXT_CSV))
        .body(csv)
}

//TODO move to repo trait
async fn get_all_states<T: ValueObject + Clone + DataPointAccess<T>>(
    api: &HomeApi,
    items: &[T],
) -> anyhow::Result<Vec<(T, DataPoint<T::ValueType>)>> {
    let mut result = vec![];

    for item in items {
        let dp = item.current_data_point(api).await?;
        result.push((item.clone(), dp));
    }

    Ok(result)
}
