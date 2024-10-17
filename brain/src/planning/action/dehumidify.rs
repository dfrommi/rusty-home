use std::ops::Not;

use goap::{Effects, Preconditions};

use crate::{
    planning::{BathroomState, HomeState},
    thing::Executable,
};
use api::{command::Command, command::PowerToggle, state::Powered};

use super::Action;

use crate::prelude::*;
use anyhow::Result;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Dehumidify {}

impl Action for Dehumidify {
    async fn start(&self) -> Result<()> {
        Command::SetPower {
            device: PowerToggle::Dehumidifier,
            power_on: true,
        }
        .execute()
        .await
    }

    async fn stop(&self) -> Result<()> {
        Command::SetPower {
            device: PowerToggle::Dehumidifier,
            power_on: false,
        }
        .execute()
        .await
    }

    async fn is_running(&self) -> bool {
        Powered::Dehumidifier.current().await.unwrap_or(false)
    }

    async fn is_enabled(&self) -> bool {
        UserControlled::Dehumidifier
            .current()
            .await
            .unwrap_or_else(|e| {
                tracing::warn!(
                    "Error getting user-controlled state, falling back to enabled: {:?}",
                    e
                );
                false
            })
            .not()
    }
}

impl Preconditions<HomeState> for Dehumidify {
    fn is_fulfilled(&self, state: &HomeState) -> bool {
        state.bathroom.risk_of_mould
    }
}

impl Effects<HomeState> for Dehumidify {
    fn apply_to(&self, state: &HomeState) -> HomeState {
        HomeState {
            bathroom: BathroomState {
                risk_of_mould: false,
            },
            ..state.clone()
        }
    }
}
