use crate::{
    automation::HeatingZone,
    command::{Command, HeatingTargetState},
    core::unit::DegreeCelsius,
    home_state::{HeatingDemandLimit, HeatingMode, SetPoint, TargetHeatingMode},
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
                let setpoints = ctx.current(SetPoint::Target(radiator))?;
                let demand_limit = ctx.current(HeatingDemandLimit::Target(radiator))?;
                let target_state = if setpoints.contains(&DegreeCelsius(0.0)) {
                    HeatingTargetState::Off
                } else {
                    HeatingTargetState::Heat {
                        target_temperature: setpoints,
                        demand_limit,
                    }
                };

                Ok(Command::SetHeating {
                    device: radiator,
                    target_state,
                })
            })
            .collect::<anyhow::Result<Vec<_>>>()?;

        //Not ideal but needed to keep the trigger going. Look for better solution with triggers
        let mode = ctx.current(TargetHeatingMode::HeatingZone(self.zone))?;

        if let HeatingMode::Manual(_, trigger_id) = mode {
            tracing::info!("Manual mode active; applying target heating demand");
            Ok(RuleResult::ExecuteTrigger(commands, trigger_id))
        } else {
            tracing::info!("Applying target heating demand");
            Ok(RuleResult::Execute(commands))
        }
    }
}
