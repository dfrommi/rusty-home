mod energy_monitor;

use std::sync::Arc;

use actix_web::web::{self};
use api::state::{CurrentPowerUsage, HeatingDemand, TotalEnergyConsumption};
use serde::Deserialize;
use support::time::{DateTime, DateTimeRange};

use crate::port::{DataPointAccess, TimeSeriesAccess};

pub fn new_routes<T>(api: Arc<T>) -> actix_web::Scope
where
    T: DataPointAccess<CurrentPowerUsage>
        + DataPointAccess<HeatingDemand>
        + TimeSeriesAccess<TotalEnergyConsumption>
        + TimeSeriesAccess<HeatingDemand>
        + 'static,
{
    web::scope("/grafana")
        .route(
            "/ds/energy/current",
            web::get().to(energy_monitor::current_power::<T>),
        )
        .route(
            "/ds/energy/total",
            web::get().to(energy_monitor::total_power::<T>),
        )
        .route(
            "/ds/heating/current",
            web::get().to(energy_monitor::current_heating::<T>),
        )
        .route(
            "/ds/heating/total",
            web::get().to(energy_monitor::total_heating::<T>),
        )
        .app_data(web::Data::from(api))
}

#[derive(Clone, Debug, Deserialize)]
struct QueryTimeRange {
    from: DateTime,
    to: DateTime,
}

impl From<QueryTimeRange> for DateTimeRange {
    fn from(val: QueryTimeRange) -> Self {
        DateTimeRange::new(val.from, val.to)
    }
}

trait DashboardDisplay {
    fn display(&self) -> &'static str;
}

impl DashboardDisplay for CurrentPowerUsage {
    fn display(&self) -> &'static str {
        match self {
            CurrentPowerUsage::Fridge => "Kühlschrank",
            CurrentPowerUsage::Dehumidifier => "Blasi",
            CurrentPowerUsage::AppleTv => "Apple TV",
            CurrentPowerUsage::Tv => "TV",
            CurrentPowerUsage::AirPurifier => "Luftfilter",
            CurrentPowerUsage::CouchLight => "Couchlicht",
            CurrentPowerUsage::Dishwasher => "Geschirrspüler",
            CurrentPowerUsage::Kettle => "Wasserkocher",
            CurrentPowerUsage::WashingMachine => "Waschmaschine",
            CurrentPowerUsage::Nuc => "Nuc",
            CurrentPowerUsage::DslModem => "DSL Modem",
            CurrentPowerUsage::InternetGateway => "Internet Gateway",
            CurrentPowerUsage::NetworkSwitch => "Network Switch",
            CurrentPowerUsage::InfraredHeater => "Infrarot-Heizung",
            CurrentPowerUsage::KitchenMultiPlug => "Küche Arbeitsplatte",
            CurrentPowerUsage::CouchPlug => "Couch-Stecker",
            CurrentPowerUsage::RoomOfRequirementsDesk => "Schreibtisch",
        }
    }
}

impl DashboardDisplay for TotalEnergyConsumption {
    fn display(&self) -> &'static str {
        match self {
            TotalEnergyConsumption::Fridge => "Kühlschrank",
            TotalEnergyConsumption::Dehumidifier => "Blasi",
            TotalEnergyConsumption::AppleTv => "Apple TV",
            TotalEnergyConsumption::Tv => "TV",
            TotalEnergyConsumption::AirPurifier => "Luftfilter",
            TotalEnergyConsumption::CouchLight => "Couchlicht",
            TotalEnergyConsumption::Dishwasher => "Geschirrspüler",
            TotalEnergyConsumption::Kettle => "Wasserkocher",
            TotalEnergyConsumption::WashingMachine => "Waschmaschine",
            TotalEnergyConsumption::Nuc => "Nuc",
            TotalEnergyConsumption::DslModem => "DSL Modem",
            TotalEnergyConsumption::InternetGateway => "Internet Gateway",
            TotalEnergyConsumption::NetworkSwitch => "Network Switch",
            TotalEnergyConsumption::InfraredHeater => "Infrarot-Heizung",
            TotalEnergyConsumption::KitchenMultiPlug => "Küche Arbeitsplatte",
            TotalEnergyConsumption::CouchPlug => "Couch-Stecker",
            TotalEnergyConsumption::RoomOfRequirementsDesk => "Schreibtisch",
        }
    }
}

impl DashboardDisplay for HeatingDemand {
    fn display(&self) -> &'static str {
        match self {
            HeatingDemand::LivingRoom => "Wohnzimmer",
            HeatingDemand::Bedroom => "Schlafzimmer",
            HeatingDemand::RoomOfRequirements => "Room of Requirements",
            HeatingDemand::Kitchen => "Küche",
            HeatingDemand::Bathroom => "Bad",
        }
    }
}
