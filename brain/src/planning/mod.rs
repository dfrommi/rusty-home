extern crate goap;

use std::ops::Not;

use action::heat::Heat;
use action::request_closing_window::RequestClosingWindow;
use action::HomeAction;
use goal::room_comfort::{RoomComfort, RoomComfortLevel};
use goal::HomeGoal;
use goap::PlanningResult;
use goap::{eval::MissedGoalsError, plan};

use anyhow::Result;
use support::unit::DegreeCelsius;

use self::action::Action;
use self::{action::dehumidify::Dehumidify, goal::prevent_mould::PreventMould};

use crate::prelude::*;

mod action;
mod goal;

#[derive(Debug, PartialEq, Eq, Clone, Hash)]
struct HomeState {
    living_room: LivingRoomState,
    bedroom: BedroomState,
    kitchen: KitchenState,
    room_of_requirements: RoomOfRequirementsState,
    bathroom: BathroomState,
}

#[derive(Debug, PartialEq, Eq, Clone, Hash)]
struct LivingRoomState {
    heating_output_remains: bool,
    temperature: DegreeCelsius,
}

#[derive(Debug, PartialEq, Eq, Clone, Hash)]
struct BedroomState {
    heating_output_remains: bool,
    temperature: DegreeCelsius,
}

#[derive(Debug, PartialEq, Eq, Clone, Hash)]
struct KitchenState {
    heating_output_remains: bool,
    temperature: DegreeCelsius,
}

#[derive(Debug, PartialEq, Eq, Clone, Hash)]
struct RoomOfRequirementsState {
    heating_output_remains: bool,
    temperature: DegreeCelsius,
}

#[derive(Debug, PartialEq, Eq, Clone, Hash)]
struct BathroomState {
    risk_of_mould: bool,
}

impl HomeState {
    pub async fn new() -> Result<Self> {
        Ok(Self {
            living_room: LivingRoomState {
                heating_output_remains: ColdAirComingIn::LivingRoom.current().await?.not(),
                temperature: Temperature::LivingRoomDoor.current().await?,
            },
            bedroom: BedroomState {
                heating_output_remains: ColdAirComingIn::Bedroom.current().await?.not(),
                temperature: Temperature::BedroomDoor.current().await?,
            },
            kitchen: KitchenState {
                heating_output_remains: ColdAirComingIn::Kitchen.current().await?.not(),
                temperature: Temperature::KitchenOuterWall.current().await?,
            },
            room_of_requirements: RoomOfRequirementsState {
                heating_output_remains: ColdAirComingIn::RoomOfRequirements.current().await?.not(),
                temperature: Temperature::RoomOfRequirementsDoor.current().await?,
            },
            bathroom: BathroomState {
                risk_of_mould: RiskOfMould::Bathroom.current().await?,
            },
        })
    }
}

pub async fn do_plan() {
    let initial_state = HomeState::new().await.expect("Error initialzing state");
    let all_actions = vec![
        HomeAction::Dehumidify(Dehumidify {}),
        HomeAction::RequestClosingWindow(RequestClosingWindow {}),
        HomeAction::Heat(Heat::LivingRoom(RoomComfortLevel::EnergySaving)),
        HomeAction::Heat(Heat::LivingRoom(RoomComfortLevel::Normal)),
        HomeAction::Heat(Heat::LivingRoom(RoomComfortLevel::Comfortable)),
        HomeAction::Heat(Heat::Bedroom(RoomComfortLevel::EnergySaving)),
        HomeAction::Heat(Heat::Bedroom(RoomComfortLevel::Normal)),
        HomeAction::Heat(Heat::Bedroom(RoomComfortLevel::Comfortable)),
        HomeAction::Heat(Heat::Kitchen(RoomComfortLevel::EnergySaving)),
        HomeAction::Heat(Heat::Kitchen(RoomComfortLevel::Normal)),
        HomeAction::Heat(Heat::Kitchen(RoomComfortLevel::Comfortable)),
        HomeAction::Heat(Heat::RoomOfRequirements(RoomComfortLevel::EnergySaving)),
        HomeAction::Heat(Heat::RoomOfRequirements(RoomComfortLevel::Normal)),
        HomeAction::Heat(Heat::RoomOfRequirements(RoomComfortLevel::Comfortable)),
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
