use crate::{
    automation::HeatingZone,
    command::Command,
    home_state::TargetHeatingDemand,
};
use r#macro::Id;

use super::{Rule, RuleEvaluationContext, RuleResult};

#[derive(Debug, Clone, Id)]
pub struct FollowTargetHeatingDemand {
    zone: HeatingZone,
}

impl FollowTargetHeatingDemand {
    pub fn new(zone: HeatingZone) -> Self {
        Self { zone }
    }
}

impl Rule for FollowTargetHeatingDemand {
    fn evaluate(&self, ctx: &RuleEvaluationContext) -> anyhow::Result<RuleResult> {
        let commands = self
            .zone
            .thermostats()
            .into_iter()
            .map(|thermostat| {
                let demand = ctx.current(TargetHeatingDemand::Thermostat(thermostat))?;
                Ok(Command::SetThermostatValveOpeningPosition {
                    device: thermostat,
                    value: demand,
                })
            })
            .collect::<anyhow::Result<Vec<_>>>()?;

        Ok(RuleResult::Execute(commands))
    }
}
