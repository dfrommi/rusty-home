use crate::device_state::{RelativeHumidity, Temperature};

#[derive(Debug, Clone, Copy)]
pub enum TadoChannel {
    Temperature(Temperature),
    RelativeHumidity(RelativeHumidity),
}

pub fn default_tado_state_config() -> Vec<(&'static str, TadoChannel)> {
    vec![
        //
        // Zone 1 — Living Room
        //
        ("1", TadoChannel::Temperature(Temperature::LivingRoomTado)),
        ("1", TadoChannel::RelativeHumidity(RelativeHumidity::LivingRoomTado)),
        //
        // Zone 2 — Bedroom
        //
        ("2", TadoChannel::Temperature(Temperature::BedroomTado)),
        ("2", TadoChannel::RelativeHumidity(RelativeHumidity::BedroomTado)),
        //
        // Zone 3 — Room of Requirements
        //
        ("3", TadoChannel::Temperature(Temperature::RoomOfRequirementsTado)),
        ("3", TadoChannel::RelativeHumidity(RelativeHumidity::RoomOfRequirementsTado)),
    ]
}
