extern crate goap;

use std::ops::Not;

use action::request_closing_window::RequestClosingWindow;
use action::HomeAction;
use goal::room_comfort::{RoomComfort, RoomComfortLevel};
use goal::HomeGoal;
use goap::PlanningResult;
use goap::{eval::MissedGoalsError, plan};

use anyhow::Result;

use self::action::Action;
use self::{action::dehumidify::Dehumidify, goal::prevent_mould::PreventMould};

use crate::prelude::*;

mod action;
mod goal;

#[derive(Debug, PartialEq, Eq, Clone, Hash)]
struct HomeState {
    risk_of_mould_in_bathroom: bool,
    heating_output_remains_in_living_room: bool,
    heating_output_remains_in_bedroom: bool,
    heating_output_remains_in_kitchen: bool,
    heating_output_remains_in_room_of_requirements: bool,
}

impl HomeState {
    pub async fn new() -> Result<Self> {
        Ok(Self {
            risk_of_mould_in_bathroom: RiskOfMould::Bathroom.current().await?,
            heating_output_remains_in_living_room: ColdAirComingIn::LivingRoom
                .current()
                .await?
                .not(),
            heating_output_remains_in_bedroom: ColdAirComingIn::Bedroom.current().await?.not(),
            heating_output_remains_in_kitchen: ColdAirComingIn::Kitchen.current().await?.not(),
            heating_output_remains_in_room_of_requirements: ColdAirComingIn::RoomOfRequirements
                .current()
                .await?
                .not(),
        })
    }
}

pub async fn do_plan() {
    let initial_state = HomeState::new().await.expect("Error initialzing state");
    let all_actions = vec![
        HomeAction::Dehumidify(Dehumidify {}),
        HomeAction::RequestClosingWindow(RequestClosingWindow {}),
    ];

    tracing::debug!("Planning with initial state {:?}", initial_state);

    let result: PlanningResult<'_, HomeState, HomeAction> = plan(
        &initial_state,
        &all_actions,
        &MissedGoalsError::new(&[
            HomeGoal::PreventMould(PreventMould {}),
            HomeGoal::RoomComfort(RoomComfort::LivingRoom(RoomComfortLevel::Comfortable)),
            HomeGoal::RoomComfort(RoomComfort::Bedroom(RoomComfortLevel::Normal)),
            HomeGoal::RoomComfort(RoomComfort::Kitchen(RoomComfortLevel::EnergySaving)),
            HomeGoal::RoomComfort(RoomComfort::RoomOfRequirements(
                RoomComfortLevel::EnergySaving,
            )),
        ]),
    );

    tracing::debug!("Planning result {:?}", result);
    tracing::debug!("Planning result next actions {:?}", result.next_actions);

    //Stop actions when leaving
    for action in &all_actions {
        let is_enabled = action.is_enabled().await;
        let is_running = action.is_running().await;
        tracing::debug!("{:?} Enabled = {:?}", action, is_enabled);

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

    //start new actions
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
