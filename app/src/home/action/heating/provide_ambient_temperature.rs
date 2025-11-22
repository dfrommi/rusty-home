use r#macro::{EnumVariants, Id};

use crate::{
    core::HomeApi,
    home::{
        HeatingZone, Thermostat,
        action::{Rule, RuleResult},
        command::Command,
    },
    port::DataPointAccess as _,
};

#[derive(Debug, Clone, PartialEq, Eq, Id, EnumVariants)]
pub enum ProvideAmbientTemperature {
    Thermostat(Thermostat),
}

impl Rule for ProvideAmbientTemperature {
    async fn evaluate(&self, api: &HomeApi) -> anyhow::Result<RuleResult> {
        let ProvideAmbientTemperature::Thermostat(thermostat) = self;

        //Sonoff thermostat is not supported
        if thermostat == &Thermostat::RoomOfRequirements {
            return Ok(RuleResult::Skip);
        }

        let temperature = HeatingZone::for_thermostat(thermostat)
            .inside_temperature()
            .current(api)
            .await?;

        Ok(RuleResult::Execute(vec![Command::SetThermostatAmbientTemperature {
            device: thermostat.clone(),
            temperature,
        }]))
    }
}
