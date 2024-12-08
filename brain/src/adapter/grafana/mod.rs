mod energy_monitor;
mod state;

use std::sync::Arc;

use actix_web::{
    http::header,
    web::{self},
    HttpResponse, ResponseError,
};
use anyhow::Context;
use api::state::{
    Channel, CurrentPowerUsage, HeatingDemand, RelativeHumidity, Temperature,
    TotalEnergyConsumption,
};
use derive_more::derive::{Display, Error};
use serde::Deserialize;
use support::time::{DateTime, DateTimeRange, Duration};

use crate::port::{DataPointAccess, TimeSeriesAccess};

#[derive(Debug, Error, Display)]
enum GrafanaApiError {
    #[display("Channel not found: type={_0} / name={_1}")]
    ChannelNotFound(String, String),

    #[display("Channel not supported: {_0:?}")]
    ChannelUnsupported(#[error(ignore)] Channel),

    #[display("Error accessing data")]
    DataAccessError(anyhow::Error),

    #[display("Internal error")]
    InternalError(anyhow::Error),
}

pub fn new_routes<T>(api: Arc<T>) -> actix_web::Scope
where
    T: DataPointAccess<CurrentPowerUsage>
        + DataPointAccess<HeatingDemand>
        + TimeSeriesAccess<TotalEnergyConsumption>
        + TimeSeriesAccess<HeatingDemand>
        + TimeSeriesAccess<Temperature>
        + TimeSeriesAccess<RelativeHumidity>
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
        .route("/ds/state", web::get().to(state::get_types))
        .route("/ds/state/{type}", web::get().to(state::get_items))
        .route(
            "/ds/state/{type}/{item}",
            web::get().to(state::state_ts::<T>),
        )
        .app_data(web::Data::from(api))
}

#[derive(Clone, Debug, Deserialize)]
struct QueryTimeRange {
    from: DateTime,
    to: DateTime,
    interval_ms: Option<i64>,
}

impl QueryTimeRange {
    fn range(&self) -> DateTimeRange {
        DateTimeRange::new(self.from, self.to)
    }

    fn interval(&self) -> Option<Duration> {
        self.interval_ms.map(Duration::millis)
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

impl ResponseError for GrafanaApiError {
    fn status_code(&self) -> actix_web::http::StatusCode {
        use actix_web::http::StatusCode;
        match self {
            GrafanaApiError::ChannelNotFound(_, _) => StatusCode::NOT_FOUND,
            GrafanaApiError::ChannelUnsupported(_) => StatusCode::BAD_REQUEST,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

fn csv_response<S: serde::Serialize>(rows: &[S]) -> Result<HttpResponse, GrafanaApiError> {
    let mut writer = csv::Writer::from_writer(vec![]);

    for row in rows {
        writer
            .serialize(row)
            .context("Error serializing row to CSV")
            .map_err(GrafanaApiError::InternalError)?;
    }

    let csv = writer
        .into_inner()
        .context("Error creating CSV")
        .map_err(GrafanaApiError::InternalError)?;

    Ok(HttpResponse::Ok()
        .append_header(header::ContentType(mime::TEXT_CSV))
        .body(csv))
}
