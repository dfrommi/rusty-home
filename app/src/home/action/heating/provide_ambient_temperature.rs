use r#macro::{EnumVariants, Id};

use crate::{
    core::HomeApi,
    home::{
        action::{Rule, RuleResult},
        command::{Command, Thermostat},
        state::Temperature,
    },
    port::DataPointAccess as _,
};

#[derive(Debug, Clone, PartialEq, Eq, Id, EnumVariants)]
pub enum ProvideAmbientTemperature {
    Thermostat(Thermostat),
}

impl Rule for ProvideAmbientTemperature {
    async fn evaluate(&self, api: &HomeApi) -> anyhow::Result<RuleResult> {
        let (thermostat, temp_sensor) = match self {
            ProvideAmbientTemperature::Thermostat(thermostat) => (
                thermostat,
                match thermostat {
                    Thermostat::LivingRoomBig | Thermostat::LivingRoomSmall => Temperature::LivingRoomDoor,
                    Thermostat::Bedroom => Temperature::BedroomOuterWall,
                    Thermostat::Kitchen => Temperature::KitchenOuterWall,
                    Thermostat::RoomOfRequirements => Temperature::RoomOfRequirementsDoor,
                    Thermostat::Bathroom => Temperature::BathroomShower,
                },
            ),
        };

        let temperature = temp_sensor.current(api).await?;

        Ok(RuleResult::Execute(vec![Command::SetThermostatAmbientTemperature {
            device: thermostat.clone(),
            temperature,
        }]))
    }
}
