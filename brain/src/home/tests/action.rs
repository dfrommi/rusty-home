use api::command::Thermostat;
use support::{
    t,
    time::{DateTime, FIXED_NOW},
    unit::DegreeCelsius,
};

use crate::{
    core::planner::{Action, ActionEvaluationResult},
    home::{
        action::{
            DeferHeatingUntilVentilationDone, ExtendHeatingUntilSleeping, HeatingZone, HomeAction,
            KeepUserOverride, NoHeatingDuringAutomaticTemperatureIncrease,
        },
        state::UserControlled,
    },
};

use super::{infrastructure, runtime};

pub struct ActionState {
    pub is_fulfilled: bool,
}

pub fn get_state_at(iso: &str, action: impl Into<HomeAction>) -> ActionState {
    let fake_now = DateTime::from_iso(iso).unwrap();
    let action: HomeAction = action.into();

    runtime().block_on(FIXED_NOW.scope(fake_now, async {
        let api = infrastructure();

        let result = action.evaluate(api).await.unwrap();

        let is_fulfilled = !matches!(result, ActionEvaluationResult::Skip);

        ActionState { is_fulfilled }
    }))
}

#[test]
fn user_override_kept_continuously() {
    let action = KeepUserOverride::new(
        UserControlled::BedroomThermostat,
        Thermostat::Bedroom.into(),
    );

    let result = get_state_at("2024-11-11T21:12:01+01:00", action);

    assert!(!result.is_fulfilled);
}

#[test]
fn heating_started_before_window_was_opened_in_one_room() {
    let action = DeferHeatingUntilVentilationDone::new(
        HeatingZone::Bedroom,
        DegreeCelsius(18.1),
        t!(6:12-12:30),
    );

    let result = get_state_at("2024-11-11T06:12:01+01:00", action);

    assert!(
            result.is_fulfilled,
            "Not fulfilled but expected. Check that window-open time is verified against date and time, not only time"
    );
}

#[test]
fn defered_heating_after_ventilation_stopped_too_early() {
    let action = NoHeatingDuringAutomaticTemperatureIncrease::new(HeatingZone::LivingRoom);
    let result = get_state_at("2024-11-16T16:57:27.8+01:00", action);

    assert!(
        result.is_fulfilled,
        "Should be fulfilled. Check handling when too few temperature mearements exist after ventilation stopped"
    );
}

#[test]
fn no_heating_during_automatic_temperature_increase_toggling() {
    let action = NoHeatingDuringAutomaticTemperatureIncrease::new(HeatingZone::LivingRoom);

    let result = get_state_at("2024-11-12T19:18:29.738113+01:00", action);

    assert!(
        !result.is_fulfilled,
        "Should not toggle. Check if properly blocked if already executed since window opened"
    );
}

#[test]
fn heating_before_sleeping_extended_over_midnight() {
    let action = ExtendHeatingUntilSleeping::new(
        HeatingZone::LivingRoom,
        DegreeCelsius(20.0),
        t!(22:30-2:30),
    );

    let result = get_state_at("2024-12-16T00:00:10+01:00", action);

    assert!(result.is_fulfilled);
}