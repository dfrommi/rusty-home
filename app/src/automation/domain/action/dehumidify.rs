use super::{RuleEvaluationContext, SimpleRule};
use crate::command::{Command, PowerToggle};
use anyhow::Result;
use r#macro::{EnumVariants, Id};

use crate::home_state::RiskOfMould;

#[derive(Debug, Clone, Id, EnumVariants)]
pub enum Dehumidify {
    Dehumidifier,
}

impl SimpleRule for Dehumidify {
    fn command(&self) -> Command {
        match self {
            Dehumidify::Dehumidifier => Command::SetPower {
                device: PowerToggle::Dehumidifier,
                power_on: true,
            },
        }
    }

    fn preconditions_fulfilled(&self, ctx: &RuleEvaluationContext) -> Result<bool> {
        match self {
            Dehumidify::Dehumidifier => ctx.current(RiskOfMould::Bathroom),
        }
    }
}
