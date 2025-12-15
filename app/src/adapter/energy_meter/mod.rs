mod http_server;
mod incoming;
mod persistence;

use crate::{
    Infrastructure,
    adapter::incoming::IncomingDataSource,
    core::persistence::Database,
};
use incoming::EnergyMeterIncomingDataSource;
use tokio::sync::broadcast;

#[derive(Debug, Clone)]
pub struct EnergyMeter;

#[derive(Debug, Clone)]
pub struct EnergyReadingAddedEvent {
    pub id: i64,
}

impl EnergyMeter {
    pub async fn new_incoming_data_source(
        infrastructure: &Infrastructure,
    ) -> impl IncomingDataSource<EnergyReadingAddedEvent, ()> + use<> {
        let db = infrastructure.database.clone();
        let rx = infrastructure.subscribe_energy_reading_events();
        EnergyMeterIncomingDataSource::new(db.clone(), rx)
    }

    pub fn new_web_service(db: Database, tx: broadcast::Sender<EnergyReadingAddedEvent>) -> actix_web::Scope {
        http_server::new_actix_web_scope(db, tx)
    }
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
