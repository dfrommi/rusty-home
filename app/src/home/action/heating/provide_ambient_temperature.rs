use r#macro::{EnumVariants, Id};

use crate::{
    core::HomeApi,
    home::{
        Thermostat,
        action::{Rule, RuleResult},
        command::Command,
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
                    Thermostat::LivingRoomBig | Thermostat::LivingRoomSmall => Temperature::LivingRoom,
                    Thermostat::Bedroom => Temperature::BedroomOuterWall,
                    Thermostat::Kitchen => Temperature::Kitchen,
                    Thermostat::RoomOfRequirements => Temperature::RoomOfRequirements,
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
