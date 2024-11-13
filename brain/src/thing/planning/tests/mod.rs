mod helper;

use api::command::Thermostat;
use helper::get_state_at;
use support::{t, unit::DegreeCelsius};

use crate::thing::{
    planning::action::{
        HeatingZone, KeepUserOverride, NoHeatingDuringAutomaticTemperatureIncrease,
    },
    state::UserControlled,
};

use super::action::DeferHeatingUntilVentilationDone;

#[test]
fn user_override_kept_continuously() {
    let action = KeepUserOverride::new(
        UserControlled::BedroomThermostat,
        Thermostat::Bedroom.into(),
    );

    let result = get_state_at("2024-11-11T21:12:01+01:00", action);

    assert!(!result.is_fulfilled);
    assert!(!result.is_running);
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
fn no_heating_during_automatic_temperature_increase_toggling() {
    let action = NoHeatingDuringAutomaticTemperatureIncrease::new(HeatingZone::LivingRoom);

    let result = get_state_at("2024-11-12T19:18:29.738113+01:00", action);

    assert!(
        !result.is_fulfilled,
        "Should not toggle. Check if properly blocked if already executed since window opened"
    );
}
