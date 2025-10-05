use crate::core::HomeApi;
use crate::home::action::SimpleRule;
use crate::home::command::{Command, PowerToggle};
use anyhow::Result;
use r#macro::{EnumVariants, Id};

use crate::home::state::RiskOfMould;

use super::DataPointAccess;

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

    async fn preconditions_fulfilled(&self, api: &HomeApi) -> Result<bool> {
        match self {
            Dehumidify::Dehumidifier => RiskOfMould::Bathroom.current(api).await,
        }
    }
}
