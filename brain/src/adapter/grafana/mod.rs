use std::{cmp::Ordering, sync::Arc};

use actix_web::{http::header, web, HttpResponse, Responder};
use api::state::{ChannelTypeInfo, CurrentPowerUsage, HeatingDemand};
use support::DataPoint;

use crate::port::DataPointAccess;

pub fn new_routes<T>(api: Arc<T>) -> actix_web::Scope
where
    T: DataPointAccess<CurrentPowerUsage> + DataPointAccess<HeatingDemand> + 'static,
{
    web::scope("/grafana")
        .route("/ds/energy/current", web::get().to(current_power::<T>))
        .route("/ds/heating/current", web::get().to(current_heating::<T>))
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
