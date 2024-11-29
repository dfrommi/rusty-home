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

impl Radiator {
    pub fn factor(&self) -> f64 {
        match self {
            Radiator::LivingRoomBig => 1.728,
            Radiator::LivingRoomSmall => 0.501,
            Radiator::Bedroom => 1.401,
            Radiator::Kitchen => 1.485,
            Radiator::RoomOfRequirements => 1.193,
            Radiator::Bathroom => 0.496,
        }
    }
}
