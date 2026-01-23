use super::Z2mCommandTarget;
use crate::automation::Radiator;
use crate::command::{CommandTarget, PowerToggle};

pub fn default_z2m_command_config() -> Vec<(CommandTarget, Z2mCommandTarget)> {
    vec![
        //
        // THERMOSTATS
        //
        (
            CommandTarget::SetThermostatValveOpeningPosition {
                device: Radiator::RoomOfRequirements,
            },
            Z2mCommandTarget::SonoffThermostat("room_of_requirements/radiator_thermostat_sonoff"),
        ),
        (
            CommandTarget::SetThermostatValveOpeningPosition {
                device: Radiator::Bathroom,
            },
            Z2mCommandTarget::SonoffThermostat("bathroom/radiator_thermostat_sonoff"),
        ),
        (
            CommandTarget::SetThermostatValveOpeningPosition {
                device: Radiator::LivingRoomBig,
            },
            Z2mCommandTarget::SonoffThermostat("living_room/radiator_thermostat_big_sonoff"),
        ),
        (
            CommandTarget::SetThermostatValveOpeningPosition {
                device: Radiator::LivingRoomSmall,
            },
            Z2mCommandTarget::SonoffThermostat("living_room/radiator_thermostat_small_sonoff"),
        ),
        (
            CommandTarget::SetThermostatValveOpeningPosition {
                device: Radiator::Bedroom,
            },
            Z2mCommandTarget::SonoffThermostat("bedroom/radiator_thermostat_sonoff"),
        ),
        (
            CommandTarget::SetThermostatValveOpeningPosition {
                device: Radiator::Kitchen,
            },
            Z2mCommandTarget::SonoffThermostat("kitchen/radiator_thermostat_sonoff"),
        ),
        //
        // POWER PLUGS
        //
        (
            CommandTarget::SetPower {
                device: PowerToggle::Dehumidifier,
            },
            Z2mCommandTarget::PowerPlug("bathroom/dehumidifier_plug"),
        ),
    ]
}
