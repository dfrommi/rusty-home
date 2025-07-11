use std::sync::Arc;

use crate::home::state::{CurrentPowerUsage, HeatingDemand, TotalEnergyConsumption};
use actix_web::web;

use crate::port::{DataPointAccess, TimeSeriesAccess};

mod current;
mod total;

pub fn routes(api: Arc<crate::core::HomeApi>) -> actix_web::Scope
{
    web::scope("/energy_monitor")
        .route("/power/current", web::get().to(current::current_power))
        .route("/power/total", web::get().to(total::total_power))
        .route("/heating/current", web::get().to(current::current_heating))
        .route("/heating/total", web::get().to(total::total_heating))
        .app_data(web::Data::from(api))
}
