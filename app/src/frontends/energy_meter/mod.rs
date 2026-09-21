use infrastructure::EventEmitter;
use r#macro::{EnumVariants, Id};

mod http_server;

#[derive(Debug, Clone)]
pub struct EnergyMeter;

impl EnergyMeter {
    pub fn new_web_service(tx: EventEmitter<EnergyReading>) -> actix_web::Scope {
        http_server::new_actix_web_scope(tx)
    }
}

#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq, Id, EnumVariants)]
pub enum EnergyMeterTarget {
    Heating(Radiator),
}

#[derive(Debug, Clone)]
pub enum EnergyReading {
    Heating(Radiator, f64),
}

impl EnergyReading {
    pub fn target(&self) -> EnergyMeterTarget {
        match self {
            Self::Heating(item, _) => EnergyMeterTarget::Heating(*item),
        }
    }
}

#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq, Id, EnumVariants)]
pub enum Radiator {
    LivingRoomBig,
    LivingRoomSmall,
    Bedroom,
    Kitchen,
    RoomOfRequirements,
    Bathroom,
}
