use std::sync::Arc;

use actix_web::web;
use api::state::{CurrentPowerUsage, HeatingDemand, TotalEnergyConsumption};

use crate::port::{DataPointAccess, TimeSeriesAccess};

mod current;
mod total;

pub fn routes<T>(api: Arc<T>) -> actix_web::Scope
where
    T: DataPointAccess<CurrentPowerUsage>
        + TimeSeriesAccess<TotalEnergyConsumption>
        + DataPointAccess<HeatingDemand>
        + TimeSeriesAccess<HeatingDemand>
        + 'static,
{
    web::scope("/energy_monitor")
        .route("/power/current", web::get().to(current::current_power::<T>))
        .route("/power/total", web::get().to(total::total_power::<T>))
        .route(
            "/heating/current",
            web::get().to(current::current_heating::<T>),
        )
        .route("/heating/total", web::get().to(total::total_heating::<T>))
        .app_data(web::Data::from(api))
}
