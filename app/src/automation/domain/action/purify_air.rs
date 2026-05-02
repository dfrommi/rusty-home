use r#macro::Id;

use super::{Rule, RuleEvaluationContext, RuleResult};
use crate::{
    command::{Command, Fan},
    core::{
        domain::RoomWithWindow,
        timeseries::DataPoint,
        unit::{AllergenIndexValue, FanAirflow, FanSpeed, Probability},
    },
    home_state::{AllergenIndex, FanActivity, Occupancy, Ventilation},
    t,
};

#[derive(Debug, Clone, Id)]
pub enum PurifyAir {
    LivingRoom,
}

impl Rule for PurifyAir {
    fn evaluate(&self, ctx: &RuleEvaluationContext) -> anyhow::Result<RuleResult> {
        let command = match self {
            PurifyAir::LivingRoom => decide_living_room(
                ctx.current_dp(FanActivity::LivingRoomAirPurifier)?,
                ctx.current_dp(Ventilation::Room(RoomWithWindow::LivingRoom))?,
                ctx.current(AllergenIndex::LivingRoom)?,
                ctx.current(Occupancy::LivingRoomCouchShort)?,
            ),
        };

        Ok(command.map_or(RuleResult::Skip, RuleResult::Execute))
    }
}

fn decide_living_room(
    current_fan_state: DataPoint<FanAirflow>,
    ventilation: DataPoint<bool>,
    allergen_index: AllergenIndexValue,
    couch_occupancy: Probability,
) -> Option<Command> {
    let speed = speed_for_couch_occupancy(couch_occupancy);

    if ventilation.value {
        tracing::info!("Living room ventilation still active; skipping air purification");
        return None;
    }

    let ventilation_done_duration = ventilation.timestamp.elapsed();
    if ventilation_done_duration < t!(15 minutes) {
        tracing::info!("Ventilation ended less than 15 minutes ago; running air purification");
        return Some(control_living_room_air_purifier(speed));
    }

    if ventilation_done_duration > t!(45 minutes) {
        tracing::info!("Ventilation ended more than 45 minutes ago; skipping air purification");
        return None;
    }

    //from here in the slot 15-45 minutes after ventilation ended

    if current_fan_state.value == FanAirflow::Off {
        tracing::info!("Air purifier was already turned off after ventilation; keeping it off");
        return None;
    }

    if allergen_index.0 <= 1 {
        tracing::info!("Allergen index is at 1; skipping air purification");
        return None;
    }

    tracing::info!("Not in post-ventilation window. Skipping air purification");
    None
}

fn speed_for_couch_occupancy(couch_occupancy: Probability) -> FanSpeed {
    if couch_occupancy.factor() > 0.7 {
        FanSpeed::Low
    } else {
        FanSpeed::Medium
    }
}

fn control_living_room_air_purifier(speed: FanSpeed) -> Command {
    Command::ControlFan {
        device: Fan::LivingRoomAirPurifier,
        speed: FanAirflow::Forward(speed),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        core::{
            timeseries::DataPoint,
            unit::{FanAirflow, FanSpeed, p},
        },
        t,
    };

    use crate::core::time::{DateTime, Duration};

    // ── helpers ──────────────────────────────────────────────────────────────

    fn minutes_ago(n: u64) -> DateTime {
        DateTime::now() - Duration::minutes(n as i64)
    }

    fn fan_on(minutes_ago: u64) -> DataPoint<FanAirflow> {
        DataPoint::new(FanAirflow::Forward(FanSpeed::Medium), self::minutes_ago(minutes_ago))
    }

    fn fan_off(mins_ago: u64) -> DataPoint<FanAirflow> {
        DataPoint::new(FanAirflow::Off, minutes_ago(mins_ago))
    }

    fn ventilation_active() -> DataPoint<bool> {
        DataPoint::new(true, t!(1 minutes ago))
    }

    fn ventilation_ended(mins_ago: u64) -> DataPoint<bool> {
        DataPoint::new(false, minutes_ago(mins_ago))
    }

    fn allergen(level: i64) -> AllergenIndexValue {
        AllergenIndexValue(level)
    }

    fn expected_command(speed: FanSpeed) -> Command {
        Command::ControlFan {
            device: Fan::LivingRoomAirPurifier,
            speed: FanAirflow::Forward(speed),
        }
    }

    // ── speed_for_couch_occupancy ─────────────────────────────────────────────

    #[test]
    fn high_couch_occupancy_selects_low_speed() {
        assert_eq!(speed_for_couch_occupancy(p(0.71)), FanSpeed::Low);
    }

    #[test]
    fn exact_occupancy_threshold_selects_low_speed() {
        // factor() > 0.7, so 0.71 → Low; 0.70 is NOT > 0.7 → Medium
        assert_eq!(speed_for_couch_occupancy(p(0.70)), FanSpeed::Medium);
    }

    #[test]
    fn low_couch_occupancy_selects_medium_speed() {
        assert_eq!(speed_for_couch_occupancy(p(0.5)), FanSpeed::Medium);
    }

    #[test]
    fn zero_couch_occupancy_selects_medium_speed() {
        assert_eq!(speed_for_couch_occupancy(p(0.0)), FanSpeed::Medium);
    }

    // ── ventilation currently active ──────────────────────────────────────────

    #[test]
    fn ventilation_active_skips_purification() {
        let result = decide_living_room(fan_off(10), ventilation_active(), allergen(3), p(0.5));
        assert!(result.is_none());
    }

    // ── ventilation ended < 15 minutes ago ───────────────────────────────────

    #[test]
    fn ventilation_ended_recently_triggers_purification() {
        let result = decide_living_room(fan_off(30), ventilation_ended(10), allergen(1), p(0.5));
        assert!(result.is_some());
    }

    #[test]
    fn ventilation_ended_recently_with_vacant_couch_uses_medium_speed() {
        let result = decide_living_room(fan_off(30), ventilation_ended(10), allergen(1), p(0.3));
        assert_eq!(result, Some(expected_command(FanSpeed::Medium)));
    }

    #[test]
    fn ventilation_ended_recently_with_occupied_couch_uses_low_speed() {
        let result = decide_living_room(fan_off(30), ventilation_ended(10), allergen(1), p(0.9));
        assert_eq!(result, Some(expected_command(FanSpeed::Low)));
    }

    // ── ventilation ended > 45 minutes ago ───────────────────────────────────

    #[test]
    fn ventilation_ended_long_ago_skips_purification() {
        let result = decide_living_room(fan_on(50), ventilation_ended(50), allergen(3), p(0.5));
        assert!(result.is_none());
    }

    // ── 15-45 minute post-ventilation window ─────────────────────────────────

    #[test]
    fn in_window_fan_already_off_stays_off() {
        let result = decide_living_room(fan_off(20), ventilation_ended(30), allergen(5), p(0.5));
        assert!(result.is_none());
    }

    #[test]
    fn in_window_fan_on_low_allergen_skips_purification() {
        let result = decide_living_room(fan_on(20), ventilation_ended(30), allergen(1), p(0.5));
        assert!(result.is_none());
    }

    #[test]
    fn in_window_fan_on_high_allergen_skips_purification() {
        // In the 15-45 min window with fan running and elevated allergens the
        // rule currently defers to other rules (returns None / Skip).
        let result = decide_living_room(fan_on(20), ventilation_ended(30), allergen(3), p(0.5));
        assert!(result.is_none());
    }

    #[test]
    fn in_window_fan_on_allergen_exactly_at_threshold_skips_purification() {
        // allergen_index.0 <= 1  →  skip; value of 1 is the boundary
        let result = decide_living_room(fan_on(20), ventilation_ended(30), allergen(1), p(0.5));
        assert!(result.is_none());
    }
}
