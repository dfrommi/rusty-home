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
    RoomOfRequirements,
}

impl std::fmt::Display for ProvideAmbientTemperature {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ProvideAmbientTemperature[{:?}]", self)
    }
}

impl Action for ProvideAmbientTemperature {
    async fn evaluate(&self, api: &HomeApi) -> anyhow::Result<ActionEvaluationResult> {
        let (temp_sensor, thermostat) = match self {
            ProvideAmbientTemperature::RoomOfRequirements => {
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
