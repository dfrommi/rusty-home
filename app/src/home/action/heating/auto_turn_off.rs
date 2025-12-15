use r#macro::{EnumVariants, Id};

use crate::command::{Command, PowerToggle};
use crate::core::time::Duration;
use crate::home::action::{Rule, RuleEvaluationContext, RuleResult};
use crate::home_state::PowerAvailable;
use crate::t;

#[derive(Debug, Clone, Id, EnumVariants)]
pub enum AutoTurnOff {
    IrHeater,
}

impl Rule for AutoTurnOff {
    fn evaluate(&self, ctx: &RuleEvaluationContext) -> anyhow::Result<RuleResult> {
        let command = match self {
            AutoTurnOff::IrHeater if on_for_at_least(PowerAvailable::InfraredHeater, t!(1 hours), ctx)? => {
                Command::SetPower {
                    device: PowerToggle::InfraredHeater,
                    power_on: false,
                }
            }
            _ => return Ok(RuleResult::Skip),
        };

        Ok(RuleResult::Execute(vec![command]))
    }
}

fn on_for_at_least(device: PowerAvailable, duration: Duration, ctx: &RuleEvaluationContext) -> anyhow::Result<bool> {
    let current = ctx.current_dp(device)?;
    Ok(current.value && current.timestamp.elapsed() > duration)
}
