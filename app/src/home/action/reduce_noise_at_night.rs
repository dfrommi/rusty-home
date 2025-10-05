use r#macro::{EnumVariants, Id};

use crate::core::HomeApi;
use crate::home::action::{Rule, RuleResult};
use crate::home::command::{Command, PowerToggle};
use crate::t;

#[derive(Debug, Clone, Id, EnumVariants)]
pub enum ReduceNoiseAtNight {
    Dehumidifier,
}

impl Rule for ReduceNoiseAtNight {
    async fn evaluate(&self, _api: &HomeApi) -> anyhow::Result<super::RuleResult> {
        let command = match self {
            ReduceNoiseAtNight::Dehumidifier if t!(22:30 - 12:00).is_now() => Command::SetPower {
                device: PowerToggle::Dehumidifier,
                power_on: false,
            },
            _ => return Ok(RuleResult::Skip),
        };

        Ok(RuleResult::Execute(vec![command]))
    }
}
