use std::{cmp::Ordering, sync::Arc};

use actix_web::{
    http::header,
    web::{self, Query},
    HttpResponse, Responder,
};
use api::state::{ChannelTypeInfo, CurrentPowerUsage, HeatingDemand, TotalEnergyConsumption};
use serde::Deserialize;
use support::{
    time::{DateTime, DateTimeRange},
    DataPoint,
};

use crate::port::{DataPointAccess, TimeSeriesAccess};

const EURO_PER_KWH: f64 = 0.349;

#[derive(Clone, Debug, Deserialize)]
struct QueryTimeRange {
    from: DateTime,
    to: DateTime,
}

pub fn new_routes<T>(api: Arc<T>) -> actix_web::Scope
where
    T: DataPointAccess<CurrentPowerUsage>
        + DataPointAccess<HeatingDemand>
        + TimeSeriesAccess<TotalEnergyConsumption>
        + TimeSeriesAccess<HeatingDemand>
        + 'static,
{
    web::scope("/grafana")
        .route("/ds/energy/current", web::get().to(current_power::<T>))
        .route("/ds/energy/total", web::get().to(total_power::<T>))
        .route("/ds/heating/current", web::get().to(current_heating::<T>))
        .route("/ds/heating/total", web::get().to(total_heating::<T>))
        .app_data(web::Data::from(api))
}

async fn current_power<T>(api: web::Data<T>) -> impl Responder
where
    T: DataPointAccess<CurrentPowerUsage>,
{
    //TODO make available on the enum
    let all_items = vec![
        CurrentPowerUsage::Fridge,
        CurrentPowerUsage::Dehumidifier,
        CurrentPowerUsage::AppleTv,
        CurrentPowerUsage::Tv,
        CurrentPowerUsage::AirPurifier,
        CurrentPowerUsage::CouchLight,
        CurrentPowerUsage::Dishwasher,
        CurrentPowerUsage::Kettle,
        CurrentPowerUsage::WashingMachine,
        CurrentPowerUsage::Nuc,
        CurrentPowerUsage::DslModem,
        CurrentPowerUsage::InternetGateway,
        CurrentPowerUsage::NetworkSwitch,
        CurrentPowerUsage::InfraredHeater,
        CurrentPowerUsage::KitchenMultiPlug,
        CurrentPowerUsage::CouchPlug,
        CurrentPowerUsage::RoomOfRequirementsDesk,
    ];

    let values = get_all_states(api.as_ref(), &all_items).await;
    if let Err(e) = values {
        return HttpResponse::InternalServerError().body(format!("Error: {}", e));
    }

    let mut values = values.unwrap();
    values.sort_by(|(_, a), (_, b)| b.value.partial_cmp(&a.value).unwrap_or(Ordering::Equal));

    let mut csv = "name,watt\n".to_string();

    for (item, dp) in values {
        let line = format!("{},{}\n", DashboardDisplay::display(&item), dp.value.0);
        csv.push_str(&line);
    }

    HttpResponse::Ok()
        .append_header(header::ContentType(mime::TEXT_CSV))
        .body(csv)
}

async fn current_heating<T>(api: web::Data<T>) -> impl Responder
where
    T: DataPointAccess<HeatingDemand>,
{
    let all_items = vec![
        HeatingDemand::LivingRoom,
        HeatingDemand::Bedroom,
        HeatingDemand::RoomOfRequirements,
        HeatingDemand::Kitchen,
        HeatingDemand::Bathroom,
    ];

    let values = get_all_states(api.as_ref(), &all_items).await;
    if let Err(e) = values {
        return HttpResponse::InternalServerError().body(format!("Error: {}", e));
    }

    let mut values = values.unwrap();
    values.sort_by(|(_, a), (_, b)| b.value.partial_cmp(&a.value).unwrap_or(Ordering::Equal));

    let mut csv = "name,percent\n".to_string();

    for (item, dp) in values {
        let line = format!("{},{}\n", DashboardDisplay::display(&item), dp.value.0);
        csv.push_str(&line);
    }

    HttpResponse::Ok()
        .append_header(header::ContentType(mime::TEXT_CSV))
        .body(csv)
}

async fn total_power<T>(api: web::Data<T>, time_range: Query<QueryTimeRange>) -> impl Responder
where
    T: TimeSeriesAccess<TotalEnergyConsumption>,
{
    struct Row {
        name: String,
        kwh: f64,
        euro: f64,
    }

    let time_range: DateTimeRange = time_range.into_inner().into();

    let all_items = vec![
        TotalEnergyConsumption::Fridge,
        TotalEnergyConsumption::Dehumidifier,
        TotalEnergyConsumption::AppleTv,
        TotalEnergyConsumption::Tv,
        TotalEnergyConsumption::AirPurifier,
        TotalEnergyConsumption::CouchLight,
        TotalEnergyConsumption::Dishwasher,
        TotalEnergyConsumption::Kettle,
        TotalEnergyConsumption::WashingMachine,
        TotalEnergyConsumption::Nuc,
        TotalEnergyConsumption::DslModem,
        TotalEnergyConsumption::InternetGateway,
        TotalEnergyConsumption::NetworkSwitch,
        TotalEnergyConsumption::InfraredHeater,
        TotalEnergyConsumption::KitchenMultiPlug,
        TotalEnergyConsumption::CouchPlug,
        TotalEnergyConsumption::RoomOfRequirementsDesk,
    ];

    let mut rows: Vec<Row> = vec![];

    for item in all_items {
        let result = api.series(item.clone(), time_range.clone()).await;
        if let Err(e) = result {
            return HttpResponse::InternalServerError()
                .body(format!("Error retrieving data: {}", e));
        }
        let result = result.unwrap();

        let value = match (result.at(time_range.start()), result.at(time_range.end())) {
            (Some(a), Some(b)) => b.value.0 - a.value.0,
            _ => {
                return HttpResponse::NotFound().body(format!(
                    "No data found for {}",
                    DashboardDisplay::display(&item)
                ))
            }
        };

        let price = value * EURO_PER_KWH;

        rows.push(Row {
            name: DashboardDisplay::display(&item).to_string(),
            kwh: value,
            euro: price,
        });
    }

    rows.sort_by(|a, b| b.kwh.partial_cmp(&a.kwh).unwrap_or(Ordering::Equal));

    let mut csv = "name,kwh,euro\n".to_string();
    for row in rows {
        csv.push_str(&format!("{},{},{}\n", row.name, row.kwh, row.euro));
    }

    HttpResponse::Ok()
        .append_header(header::ContentType(mime::TEXT_CSV))
        .body(csv)
}

async fn total_heating<T>(api: web::Data<T>, time_range: Query<QueryTimeRange>) -> impl Responder
where
    T: TimeSeriesAccess<HeatingDemand>,
{
    struct Row {
        name: String,
        value: f64,
    }

    let time_range: DateTimeRange = time_range.into_inner().into();

    let all_items = vec![
        HeatingDemand::LivingRoom,
        HeatingDemand::Bedroom,
        HeatingDemand::RoomOfRequirements,
        HeatingDemand::Kitchen,
        HeatingDemand::Bathroom,
    ];

    let mut rows: Vec<Row> = vec![];

    for item in all_items {
        let result = api.series(item.clone(), time_range.clone()).await;

        if let Err(e) = result {
            return HttpResponse::InternalServerError()
                .body(format!("Error retrieving data: {}", e));
        }
        let result = result.unwrap();

        let value = result.area_in_type_hours() + heating_factor(&item);

        rows.push(Row {
            name: DashboardDisplay::display(&item).to_string(),
            value,
        });
    }

    rows.sort_by(|a, b| b.value.partial_cmp(&a.value).unwrap_or(Ordering::Equal));
    let mut csv = "name,value\n".to_string();
    for row in rows {
        csv.push_str(&format!("{},{}\n", row.name, row.value));
    }

    HttpResponse::Ok()
        .append_header(header::ContentType(mime::TEXT_CSV))
        .body(csv)
}

fn heating_factor(item: &HeatingDemand) -> f64 {
    match item {
        HeatingDemand::LivingRoom => 1.728 + 0.501,
        HeatingDemand::Bedroom => 1.401,
        HeatingDemand::RoomOfRequirements => 1.193,
        HeatingDemand::Kitchen => 1.485,
        HeatingDemand::Bathroom => 0.496,
    }
}

//TODO move to repo trait
async fn get_all_states<T: ChannelTypeInfo + Clone>(
    api: &impl DataPointAccess<T>,
    items: &[T],
) -> anyhow::Result<Vec<(T, DataPoint<T::ValueType>)>> {
    let mut result = vec![];

    for item in items {
        let dp = api.current_data_point(item.clone()).await?;
        result.push((item.clone(), dp));
    }

    Ok(result)
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
