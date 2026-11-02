use r#macro::Id;

use super::{Rule, RuleEvaluationContext, RuleResult};
use crate::{
    command::{Command, ControlFan, Fan, PowerToggle, SetPower},
    core::{
        domain::{HeatingZone, RoomWithWindow},
        timeseries::DataPoint,
        unit::{FanAirflow, FanSpeed, Probability},
    },
    home_state::{HeatingMode, Occupancy, TargetHeatingMode, Ventilation},
    t,
};

#[derive(Debug, Clone, Id)]
pub enum PurifyAir {
    BedroomAfterVentilation,
    BedroomSleepPreparation,
    LivingRoom,
}

impl Rule for PurifyAir {
    fn evaluate(&self, ctx: &RuleEvaluationContext) -> anyhow::Result<RuleResult> {
        let command: Option<Command> = match self {
            PurifyAir::BedroomAfterVentilation => {
                decide_bedroom_after_ventilation(ctx.current_dp(Ventilation::Room(RoomWithWindow::Bedroom))?)
                    .map(Command::from)
            }
            PurifyAir::BedroomSleepPreparation => {
                decide_bedroom_sleep(ctx.current_dp(TargetHeatingMode::HeatingZone(HeatingZone::Bedroom))?)
                    .map(Command::from)
            }
            PurifyAir::LivingRoom => decide_living_room(
                ctx.current_dp(Ventilation::Room(RoomWithWindow::LivingRoom))?,
                ctx.current(Occupancy::LivingRoomCouchShort)?,
            )
            .map(Command::from),
        };

        Ok(command.map_or(RuleResult::Skip, RuleResult::Execute))
    }
}

fn decide_bedroom_after_ventilation(ventilation: DataPoint<bool>) -> Option<SetPower> {
    if ventilation.value {
        tracing::info!("Bedroom ventilation still active; skipping air purification");
        return None;
    }

    let ventilation_done_duration = ventilation.timestamp.elapsed();
    if ventilation_done_duration < t!(15 minutes) {
        tracing::info!("Bedroom ventilation ended less than 15 minutes ago; turning on air purification");
        return Some(SetPower {
            device: PowerToggle::BedroomAirPurifier,
            power_on: true,
        });
    }

    tracing::info!("Bedroom ventilation ended more than 15 minutes ago; skipping air purification");
    None
}

fn decide_bedroom_sleep(heating_mode: DataPoint<HeatingMode>) -> Option<SetPower> {
    if heating_mode.value != HeatingMode::Sleep {
        tracing::info!("Bedroom heating is not entering sleep mode; skipping air purification");
        return None;
    }

    if heating_mode.timestamp.elapsed() >= t!(15 minutes) {
        tracing::info!("Bedroom heating entered sleep mode more than 15 minutes ago; skipping air purification");
        return None;
    }

    tracing::info!("Bedroom heating entered sleep mode less than 15 minutes ago; turning on air purification");
    Some(SetPower {
        device: PowerToggle::BedroomAirPurifier,
        power_on: true,
    })
}

fn decide_living_room(ventilation: DataPoint<bool>, couch_occupancy: Probability) -> Option<ControlFan> {
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

    tracing::info!("Ventilation ended more than 15 minutes ago; skipping air purification");
    None
}

fn speed_for_couch_occupancy(couch_occupancy: Probability) -> FanSpeed {
    if couch_occupancy.factor() > 0.7 {
        FanSpeed::Low
    } else {
        FanSpeed::Medium
    }
}

fn control_living_room_air_purifier(speed: FanSpeed) -> ControlFan {
    ControlFan {
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

    fn ventilation_active() -> DataPoint<bool> {
        DataPoint::new(true, t!(1 minutes ago))
    }

    fn ventilation_ended(mins_ago: u64) -> DataPoint<bool> {
        DataPoint::new(false, minutes_ago(mins_ago))
    }

    fn heating_mode(mode: HeatingMode, mins_ago: u64) -> DataPoint<HeatingMode> {
        DataPoint::new(mode, minutes_ago(mins_ago))
    }

    fn expected_command(speed: FanSpeed) -> ControlFan {
        ControlFan {
            device: Fan::LivingRoomAirPurifier,
            speed: FanAirflow::Forward(speed),
        }
    }

    // ── bedroom purifier ──────────────────────────────────────────────────────

    #[test]
    fn bedroom_ventilation_ended_recently_turns_on_purifier() {
        let result = decide_bedroom_after_ventilation(ventilation_ended(10));

        assert_eq!(
            result,
            Some(SetPower {
                device: PowerToggle::BedroomAirPurifier,
                power_on: true,
            })
        );
    }

    #[test]
    fn bedroom_ventilation_ended_long_ago_skips_purification() {
        assert!(decide_bedroom_after_ventilation(ventilation_ended(50)).is_none());
    }

    // ── bedroom sleep mode ────────────────────────────────────────────────────

    #[test]
    fn bedroom_sleep_mode_started_recently_turns_on_purifier() {
        let result = decide_bedroom_sleep(heating_mode(HeatingMode::Sleep, 10));

        assert_eq!(
            result,
            Some(SetPower {
                device: PowerToggle::BedroomAirPurifier,
                power_on: true,
            })
        );
    }

    #[test]
    fn bedroom_sleep_mode_started_more_than_15_minutes_ago_skips_purification() {
        assert!(decide_bedroom_sleep(heating_mode(HeatingMode::Sleep, 16)).is_none());
    }

    #[test]
    fn bedroom_non_sleep_heating_mode_skips_purification() {
        assert!(decide_bedroom_sleep(heating_mode(HeatingMode::EnergySaving, 1)).is_none());
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
        let result = decide_living_room(ventilation_active(), p(0.5));
        assert!(result.is_none());
    }

    // ── ventilation ended < 15 minutes ago ───────────────────────────────────

    #[test]
    fn ventilation_ended_recently_triggers_purification() {
        let result = decide_living_room(ventilation_ended(10), p(0.5));
        assert!(result.is_some());
    }

    #[test]
    fn ventilation_ended_recently_with_vacant_couch_uses_medium_speed() {
        let result = decide_living_room(ventilation_ended(10), p(0.3));
        assert_eq!(result, Some(expected_command(FanSpeed::Medium)));
    }

    #[test]
    fn ventilation_ended_recently_with_occupied_couch_uses_low_speed() {
        let result = decide_living_room(ventilation_ended(10), p(0.9));
        assert_eq!(result, Some(expected_command(FanSpeed::Low)));
    }

    // ── ventilation ended >= 15 minutes ago ──────────────────────────────────

    #[test]
    fn ventilation_ended_long_ago_skips_purification() {
        let result = decide_living_room(ventilation_ended(50), p(0.5));
        assert!(result.is_none());
    }
}
