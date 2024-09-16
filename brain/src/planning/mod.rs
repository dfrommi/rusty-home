extern crate goap;

use goap::PlanningResult;
use goap::{eval::MissedGoalsError, plan};

use crate::error::Result;

use self::action::Action;
use self::{action::dehumidify::Dehumidify, goal::prevent_mould::PreventMouldGoal};

use crate::prelude::*;

mod action;
mod goal;

#[derive(Debug, PartialEq, Eq, Clone, Hash)]
struct HomeState {
    risk_of_mould_in_bathroom: bool,
}

impl HomeState {
    pub async fn new() -> Result<Self> {
        Ok(Self {
            risk_of_mould_in_bathroom: RiskOfMould::Bathroom.current().await?,
        })
    }
}

pub async fn do_plan() {
    let initial_state = HomeState::new().await.expect("Error initialzing state");
    let all_actions: Vec<Dehumidify> = vec![Dehumidify {}];

    tracing::debug!("Planning with initial state {:?}", initial_state);

    let result: PlanningResult<'_, HomeState, Dehumidify> = plan(
        &initial_state,
        &all_actions,
        &MissedGoalsError::new(&[PreventMouldGoal]),
    );

    tracing::debug!("Planning result next actions {:?}", result.next_actions);

    for action in &all_actions {
        let is_enabled = action.is_enabled().await;
        let is_running = action.is_running().await;
        tracing::debug!("Enabled = {:?}", is_enabled);

        if result.next_actions.contains(&action) || !is_running {
            continue;
        }

        if !is_enabled {
            tracing::debug!("Not stopping action {:?} because it's disabled", action);
            continue;
        }

        match action.stop().await {
            Ok(_) => tracing::info!("Stopped action {:?}", action),
            Err(err) => tracing::error!("Error stopping action {:?}: {}", action, err),
        };
    }

    for action in result.next_actions {
        if !action.is_enabled().await {
            tracing::debug!("Skipping action {:?} because it's disabled", action);
            continue;
        }

        match action.start().await {
            Ok(_) => tracing::info!("Action {:?} started", action),
            Err(err) => tracing::error!("Error starting action {:?}: {}", action, err),
        };
    }
}
