use super::Z2mCommandTarget;
use crate::automation::Thermostat;
use crate::command::CommandTarget;

pub fn default_z2m_command_config() -> Vec<(CommandTarget, Z2mCommandTarget)> {
    vec![
        (
            CommandTarget::SetThermostatValveOpeningPosition {
                device: Thermostat::RoomOfRequirements,
            },
            Z2mCommandTarget::Thermostat("room_of_requirements/radiator_thermostat_sonoff"),
        ),
        (
            CommandTarget::SetThermostatValveOpeningPosition {
                device: Thermostat::Bathroom,
            },
            Z2mCommandTarget::Thermostat("bathroom/radiator_thermostat_sonoff"),
        ),
        (
            CommandTarget::SetThermostatValveOpeningPosition {
                device: Thermostat::LivingRoomBig,
            },
            Z2mCommandTarget::Thermostat("living_room/radiator_thermostat_big_sonoff"),
        ),
        (
            CommandTarget::SetThermostatValveOpeningPosition {
                device: Thermostat::LivingRoomSmall,
            },
            Z2mCommandTarget::Thermostat("living_room/radiator_thermostat_small_sonoff"),
        ),
        (
            CommandTarget::SetThermostatValveOpeningPosition {
                device: Thermostat::Bedroom,
            },
            Z2mCommandTarget::Thermostat("bedroom/radiator_thermostat_sonoff"),
        ),
        (
            CommandTarget::SetThermostatValveOpeningPosition {
                device: Thermostat::Kitchen,
            },
            Z2mCommandTarget::Thermostat("kitchen/radiator_thermostat_sonoff"),
        ),
    ]
}
