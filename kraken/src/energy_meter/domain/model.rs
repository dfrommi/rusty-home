#[derive(Debug, Clone)]
pub enum EnergyReading {
    Heating(Radiator, f64),
    ColdWater(Faucet, f64),
    HotWater(Faucet, f64),
}

#[derive(Debug, Clone)]
pub enum EnergyReadingType {
    Heating,
    ColdWater,
    HotWater,
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
