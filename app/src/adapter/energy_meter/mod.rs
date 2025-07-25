mod http_server;
mod incoming;
mod persistence;

use crate::{
    core::persistence::Database,
    core::{HomeApi, app_event::EnergyReadingAddedEvent, process_incoming_data_source},
};
use incoming::EnergyMeterIncomingDataSource;
use tokio::sync::broadcast::Receiver;

#[derive(Debug, Clone)]
pub struct EnergyMeter;

impl EnergyMeter {
    pub async fn new_incoming_data_processor(
        &self,
        db: Database,
        rx: Receiver<EnergyReadingAddedEvent>,
    ) -> impl Future<Output = ()> + use<> {
        async move {
            let ds = EnergyMeterIncomingDataSource::new(db.clone(), rx);
            let api = HomeApi::new(db);
            process_incoming_data_source("EnergyReading", ds, &api).await
        }
    }
}

pub fn new_web_service(db: Database) -> actix_web::Scope {
    http_server::new_actix_web_scope(db)
}

#[derive(Debug, Clone)]
pub enum EnergyReading {
    Heating(Radiator, f64),
    ColdWater(Faucet, f64),
    HotWater(Faucet, f64),
}

#[derive(Debug, Clone)]
pub enum Radiator {
    LivingRoomBig,
    LivingRoomSmall,
    Bedroom,
    Kitchen,
    RoomOfRequirements,
    Bathroom,
}

#[derive(Debug, Clone)]
pub enum Faucet {
    Kitchen,
    Bathroom,
}
