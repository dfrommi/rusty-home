use super::Z2mCommandTarget;
use crate::automation::{LoadBalancedThermostat, Thermostat};
use crate::command::CommandTarget;

pub fn default_z2m_command_config() -> Vec<(CommandTarget, Z2mCommandTarget)> {
    vec![
        (
            CommandTarget::SetHeating {
                device: Thermostat::LivingRoomBig,
            },
            Z2mCommandTarget::Thermostat("living_room/radiator_thermostat_big"),
        ),
        (
            CommandTarget::SetHeating {
                device: Thermostat::LivingRoomSmall,
            },
            Z2mCommandTarget::Thermostat("living_room/radiator_thermostat_small"),
        ),
        (
            CommandTarget::SetHeating {
                device: Thermostat::Bedroom,
            },
            Z2mCommandTarget::Thermostat("bedroom/radiator_thermostat"),
        ),
        (
            CommandTarget::SetHeating {
                device: Thermostat::Kitchen,
            },
            Z2mCommandTarget::Thermostat("kitchen/radiator_thermostat"),
        ),
        // (
        //     CommandTarget::SetHeating {
        //         device: Thermostat::RoomOfRequirements,
        //     },
        //     Z2mCommandTarget::Thermostat("room_of_requirements/radiator_thermostat"),
        // ),
        (
            CommandTarget::SetThermostatValveOpeningPosition {
                device: Thermostat::RoomOfRequirements,
            },
            Z2mCommandTarget::Thermostat("room_of_requirements/radiator_thermostat_sonoff"),
        ),
        (
            CommandTarget::SetHeating {
                device: Thermostat::Bathroom,
            },
            Z2mCommandTarget::Thermostat("bathroom/radiator_thermostat"),
        ),
        (
            CommandTarget::SetThermostatAmbientTemperature {
                device: Thermostat::LivingRoomBig,
            },
            Z2mCommandTarget::Thermostat("living_room/radiator_thermostat_big"),
        ),
        (
            CommandTarget::SetThermostatAmbientTemperature {
                device: Thermostat::LivingRoomSmall,
            },
            Z2mCommandTarget::Thermostat("living_room/radiator_thermostat_small"),
        ),
        (
            CommandTarget::SetThermostatLoadMean {
                device: LoadBalancedThermostat::LivingRoomBig,
            },
            Z2mCommandTarget::Thermostat("living_room/radiator_thermostat_big"),
        ),
        (
            CommandTarget::SetThermostatLoadMean {
                device: LoadBalancedThermostat::LivingRoomSmall,
            },
            Z2mCommandTarget::Thermostat("living_room/radiator_thermostat_small"),
        ),
        (
            CommandTarget::SetThermostatAmbientTemperature {
                device: Thermostat::Bedroom,
            },
            Z2mCommandTarget::Thermostat("bedroom/radiator_thermostat"),
        ),
        (
            CommandTarget::SetThermostatAmbientTemperature {
                device: Thermostat::Kitchen,
            },
            Z2mCommandTarget::Thermostat("kitchen/radiator_thermostat"),
        ),
        // (
        //     CommandTarget::SetThermostatAmbientTemperature {
        //         device: Thermostat::RoomOfRequirements,
        //     },
        //     Z2mCommandTarget::Thermostat("room_of_requirements/radiator_thermostat"),
        // ),
        (
            CommandTarget::SetThermostatAmbientTemperature {
                device: Thermostat::Bathroom,
            },
            Z2mCommandTarget::Thermostat("bathroom/radiator_thermostat"),
        ),
    ]
}
