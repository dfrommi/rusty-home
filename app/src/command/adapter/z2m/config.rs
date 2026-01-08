use super::Z2mCommandTarget;
use crate::automation::Thermostat;
use crate::command::CommandTarget;

pub fn default_z2m_command_config() -> Vec<(CommandTarget, Z2mCommandTarget)> {
    vec![
        (
            CommandTarget::SetThermostatValveOpeningPosition {
                device: Thermostat::RoomOfRequirements,
            },
            Z2mCommandTarget::SonoffThermostat("room_of_requirements/radiator_thermostat_sonoff"),
        ),
        (
            CommandTarget::SetThermostatValveOpeningPosition {
                device: Thermostat::Bathroom,
            },
            Z2mCommandTarget::SonoffThermostat("bathroom/radiator_thermostat_sonoff"),
        ),
        (
            CommandTarget::SetThermostatValveOpeningPosition {
                device: Thermostat::LivingRoomBig,
            },
            Z2mCommandTarget::SonoffThermostat("living_room/radiator_thermostat_big_sonoff"),
        ),
        (
            CommandTarget::SetThermostatValveOpeningPosition {
                device: Thermostat::LivingRoomSmall,
            },
            Z2mCommandTarget::SonoffThermostat("living_room/radiator_thermostat_small_sonoff"),
        ),
        (
            CommandTarget::SetThermostatValveOpeningPosition {
                device: Thermostat::Bedroom,
            },
            Z2mCommandTarget::SonoffThermostat("bedroom/radiator_thermostat_sonoff"),
        ),
        (
            CommandTarget::SetThermostatValveOpeningPosition {
                device: Thermostat::Kitchen,
            },
            Z2mCommandTarget::SonoffThermostat("kitchen/radiator_thermostat_sonoff"),
        ),
    ]
}
