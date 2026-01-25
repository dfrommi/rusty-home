use r#macro::{EnumVariants, Id};

use crate::{
    automation::{
        HeatingZone, Room, RuleEvaluationContext,
        domain::action::{FollowDefaultSetting, Rule, RuleResult, UserTriggerAction},
    },
    command::{CommandTarget, Fan, PowerToggle},
    core::unit::DegreeCelsius,
    frontends::homekit::HomekitCommandTarget::{self},
    home_state::{TargetHeatingMode, Temperature},
    t,
    trigger::UserTriggerTarget,
};

#[derive(Debug, Clone, Id, EnumVariants)]
pub enum BlockAutomation {
    BathroomDehumidifier,
    BedroomDehumidifier,
    BedroomCeilingFan,
}

impl Rule for BlockAutomation {
    fn evaluate(&self, ctx: &RuleEvaluationContext) -> anyhow::Result<RuleResult> {
        let sleeping_start = {
            let mode = ctx.current_dp(TargetHeatingMode::HeatingZone(HeatingZone::Bedroom))?;
            if mode.value == crate::home_state::HeatingMode::Sleep {
                Some(mode.timestamp)
            } else {
                None
            }
        };

        //None -> not blocked
        let blocked_start = match self {
            BlockAutomation::BathroomDehumidifier | BlockAutomation::BedroomDehumidifier => {
                //TODO and not sleeping
                let night_time_start = t!(22:00 - 9:00).active().map(|r| *r.start());
                if sleeping_start.is_some() || night_time_start.is_some() {
                    tracing::info!("Sleep mode or night time active; blocking dehumidifier");
                } else {
                    tracing::info!("Not sleeping and not night time; not blocking dehumidifier");
                }
                //min of both
                sleeping_start
                    .map(|s| s.min(night_time_start.unwrap_or(s)))
                    .or(night_time_start)
            }
            BlockAutomation::BedroomCeilingFan => {
                let room_temp = ctx.current(Temperature::Room(Room::Bedroom))?;
                if room_temp >= DegreeCelsius(25.0) {
                    tracing::info!("Bedroom temperature at least 25C; not blocking ceiling fan");
                    None
                } else if let Some(start) = sleeping_start {
                    tracing::info!("Sleep mode active; blocking ceiling fan");
                    Some(start)
                } else {
                    tracing::info!("Not in sleep mode; not blocking ceiling fan");
                    None
                }
            }
        };

        let Some(blocked_start) = blocked_start else {
            tracing::info!("No block active; skipping");
            return Ok(RuleResult::Skip);
        };

        //Execute trigger if started after block started. Then user intentionally want this.

        let (trigger_target, command_target) = match self {
            BlockAutomation::BathroomDehumidifier => (
                UserTriggerTarget::Homekit(HomekitCommandTarget::DehumidifierPower),
                CommandTarget::SetPower {
                    device: PowerToggle::Dehumidifier,
                },
            ),
            BlockAutomation::BedroomDehumidifier => (
                UserTriggerTarget::Homekit(HomekitCommandTarget::BedroomDehumidifierFanSpeed),
                CommandTarget::ControlFan {
                    device: Fan::BedroomDehumidifier,
                },
            ),
            BlockAutomation::BedroomCeilingFan => (
                UserTriggerTarget::Homekit(HomekitCommandTarget::BedroomCeilingFanSpeed),
                CommandTarget::ControlFan {
                    device: Fan::BedroomCeilingFan,
                },
            ),
        };

        let trigger = ctx.latest_trigger(trigger_target);

        //User-trigger after night time starts -> user really wants to override
        if let Some(trigger) = trigger
            && trigger.timestamp > blocked_start
        {
            tracing::info!("User override detected; delegating to user trigger");
            let result = UserTriggerAction::new(trigger.target().clone()).evaluate(ctx);
            if matches!(result, Ok(RuleResult::ExecuteTrigger(..))) {
                return result;
            }
        }

        tracing::info!("No valid user override; applying default setting");
        FollowDefaultSetting::new(command_target).evaluate(ctx)
    }
}
