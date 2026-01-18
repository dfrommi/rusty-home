use crate::{
    automation::HeatingZone,
    command::Command,
    home_state::{HeatingMode, TargetHeatingDemand, TargetHeatingMode},
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
        let radiators = self.zone.radiators();

        let commands = radiators
            .into_iter()
            .map(|radiator| {
                let demand = ctx.current(TargetHeatingDemand::ControlAndObserve(radiator))?;
                Ok(Command::SetThermostatValveOpeningPosition {
                    device: radiator,
                    value: demand,
                })
            })
            .collect::<anyhow::Result<Vec<_>>>()?;

        //Not ideal but needed to keep the trigger going. Look for better solution with triggers
        let mode = ctx.current(TargetHeatingMode::HeatingZone(self.zone))?;

        if let HeatingMode::Manual(_, trigger_id) = mode {
            Ok(RuleResult::ExecuteTrigger(commands, trigger_id))
        } else {
            Ok(RuleResult::Execute(commands))
        }
    }
}
