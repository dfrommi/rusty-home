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
    LivingRoomThermostatSmall,
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
            ProvideAmbientTemperature::LivingRoomThermostatBig => {
                (Temperature::LivingRoomDoor, Thermostat::LivingRoomBig)
            }
            ProvideAmbientTemperature::LivingRoomThermostatSmall => {
                (Temperature::LivingRoomDoor, Thermostat::LivingRoomSmall)
            }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_includes_variant_name() {
        let cases = [
            (
                ProvideAmbientTemperature::LivingRoomThermostatBig,
                "ProvideAmbientTemperature[LivingRoomThermostatBig]",
            ),
            (
                ProvideAmbientTemperature::LivingRoomThermostatSmall,
                "ProvideAmbientTemperature[LivingRoomThermostatSmall]",
            ),
            (
                ProvideAmbientTemperature::BedroomThermostat,
                "ProvideAmbientTemperature[BedroomThermostat]",
            ),
            (
                ProvideAmbientTemperature::KitchenThermostat,
                "ProvideAmbientTemperature[KitchenThermostat]",
            ),
            (
                ProvideAmbientTemperature::RoomOfRequirementsThermostat,
                "ProvideAmbientTemperature[RoomOfRequirementsThermostat]",
            ),
        ];

        for (variant, expected) in cases {
            assert_eq!(variant.to_string(), expected);
        }
    }
}
