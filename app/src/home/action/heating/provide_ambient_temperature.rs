use r#macro::{EnumVariants, Id};

use crate::home::{
    HeatingZone, Thermostat,
    action::{Rule, RuleEvaluationContext, RuleResult},
};
use crate::command::Command;

#[derive(Debug, Clone, PartialEq, Eq, Id, EnumVariants)]
pub enum ProvideAmbientTemperature {
    Thermostat(Thermostat),
}

impl Rule for ProvideAmbientTemperature {
    fn evaluate(&self, ctx: &RuleEvaluationContext) -> anyhow::Result<RuleResult> {
        let ProvideAmbientTemperature::Thermostat(thermostat) = self;

        //Sonoff thermostat is not supported
        if thermostat == &Thermostat::RoomOfRequirements {
            return Ok(RuleResult::Skip);
        }

        let temperature = ctx.current(HeatingZone::for_thermostat(thermostat).inside_temperature())?;

        Ok(RuleResult::Execute(vec![Command::SetThermostatAmbientTemperature {
            device: thermostat.clone(),
            temperature,
        }]))
    }
}
