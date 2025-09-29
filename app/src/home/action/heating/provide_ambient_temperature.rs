use crate::{
    core::{
        HomeApi,
        planner::{Action, ActionEvaluationResult},
    },
    home::{
        command::{Command, Thermostat},
        state::Temperature,
    },
    port::DataPointAccess as _,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProvideAmbientTemperature {
    LivingRoomThermostatBig,
    BedroomThermostat,
    KitchenThermostat,
    RoomOfRequirementsThermostat,
}

impl std::fmt::Display for ProvideAmbientTemperature {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ProvideAmbientTemperature[{:?}]", self)
    }
}

impl Action for ProvideAmbientTemperature {
    async fn evaluate(&self, api: &HomeApi) -> anyhow::Result<ActionEvaluationResult> {
        let (temp_sensor, thermostat) = match self {
            ProvideAmbientTemperature::LivingRoomThermostatBig => (Temperature::LivingRoomDoor, Thermostat::LivingRoom),
            ProvideAmbientTemperature::BedroomThermostat => (Temperature::BedroomDoor, Thermostat::Bedroom),
            ProvideAmbientTemperature::KitchenThermostat => (Temperature::KitchenOuterWall, Thermostat::Kitchen),
            ProvideAmbientTemperature::RoomOfRequirementsThermostat => {
                (Temperature::RoomOfRequirementsDoor, Thermostat::RoomOfRequirements)
            }
        };

        let temperature = temp_sensor.current(api).await?;

        Ok(ActionEvaluationResult::Execute(
            Command::SetThermostatAmbientTemperature {
                device: thermostat,
                temperature,
            },
            super::action_source(self),
        ))
    }
}
