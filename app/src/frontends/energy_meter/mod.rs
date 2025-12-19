use infrastructure::EventEmitter;

mod http_server;

#[derive(Debug, Clone)]
pub struct EnergyMeter;

impl EnergyMeter {
    pub fn new_web_service(tx: EventEmitter<EnergyReading>) -> actix_web::Scope {
        http_server::new_actix_web_scope(tx)
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
