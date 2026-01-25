use r#macro::{EnumVariants, Id};

use super::{Rule, RuleEvaluationContext, RuleResult};
use crate::command::{Command, PowerToggle};
use crate::core::time::Duration;
use crate::home_state::PowerAvailable;
use crate::t;

#[derive(Debug, Clone, Id, EnumVariants)]
pub enum AutoTurnOff {
    IrHeater,
}

impl Rule for AutoTurnOff {
    fn evaluate(&self, ctx: &RuleEvaluationContext) -> anyhow::Result<RuleResult> {
        let should_turn_off = match self {
            AutoTurnOff::IrHeater => on_for_at_least(PowerAvailable::InfraredHeater, t!(1 hours), ctx)?,
        };

        if should_turn_off {
            tracing::info!("Infrared heater on for more than 1 hour; turning off");
            Ok(RuleResult::Execute(vec![Command::SetPower {
                device: PowerToggle::InfraredHeater,
                power_on: false,
            }]))
        } else {
            tracing::info!("Infrared heater not on for more than 1 hour; skipping");
            Ok(RuleResult::Skip)
        }
    }
}

fn on_for_at_least(device: PowerAvailable, duration: Duration, ctx: &RuleEvaluationContext) -> anyhow::Result<bool> {
    let current = ctx.current_dp(device)?;
    Ok(current.value && current.timestamp.elapsed() > duration)
}
