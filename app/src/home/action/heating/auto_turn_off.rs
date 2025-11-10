use r#macro::{EnumVariants, Id};

use crate::core::HomeApi;
use crate::core::time::Duration;
use crate::home::action::{Rule, RuleResult};
use crate::home::command::{Command, PowerToggle};
use crate::home::state::PowerAvailable;
use crate::t;

use crate::port::DataPointAccess;

#[derive(Debug, Clone, Id, EnumVariants)]
pub enum AutoTurnOff {
    IrHeater,
}

impl Rule for AutoTurnOff {
    async fn evaluate(&self, api: &HomeApi) -> anyhow::Result<RuleResult> {
        let command = match self {
            AutoTurnOff::IrHeater if on_for_at_least(PowerAvailable::InfraredHeater, t!(1 hours), api).await? => {
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

async fn on_for_at_least(device: PowerAvailable, duration: Duration, api: &HomeApi) -> anyhow::Result<bool> {
    let current = device.current_data_point(api).await?;
    Ok(current.value && current.timestamp.elapsed() > duration)
}
