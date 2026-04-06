use crate::{
    command::{Command, HeatingTargetState},
    core::domain::Radiator,
    core::unit::DegreeCelsius,
    home_state::{HeatingDemandLimit, HeatingMode, SetPoint, TargetHeatingMode},
};
use r#macro::Id;

use super::{Rule, RuleEvaluationContext, RuleResult};

#[derive(Debug, Clone, Id)]
pub struct FollowTargetHeatingDemand {
    radiator: Radiator,
}

impl FollowTargetHeatingDemand {
    pub fn new(radiator: Radiator) -> Self {
        Self { radiator }
    }
}

impl Rule for FollowTargetHeatingDemand {
    fn evaluate(&self, ctx: &RuleEvaluationContext) -> anyhow::Result<RuleResult> {
        let setpoints = ctx.current(SetPoint::Target(self.radiator))?;
        let demand_limit = ctx.current(HeatingDemandLimit::Target(self.radiator))?;
        let target_state = if setpoints.contains(&DegreeCelsius(0.0)) {
            HeatingTargetState::Off
        } else {
            HeatingTargetState::Heat {
                target_temperature: setpoints,
                demand_limit,
            }
        };

        let command = Command::SetHeating {
            device: self.radiator,
            target_state,
        };

        let zone = self.radiator.heating_zone();
        let mode = ctx.current(TargetHeatingMode::HeatingZone(zone))?;

        if let HeatingMode::Manual(_, trigger_id) = mode {
            tracing::info!("Manual mode active; applying target heating demand");
            Ok(RuleResult::ExecuteTrigger(command, trigger_id))
        } else {
            tracing::info!("Applying target heating demand");
            Ok(RuleResult::Execute(command))
        }
    }
}
