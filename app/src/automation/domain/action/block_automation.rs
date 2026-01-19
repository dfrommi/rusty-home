use r#macro::{EnumVariants, Id};

use crate::{
    automation::{
        RuleEvaluationContext,
        domain::action::{FollowDefaultSetting, Rule, RuleResult, UserTriggerAction},
    },
    command::{Fan, PowerToggle},
    frontends::homekit::HomekitCommandTarget::{self},
    t,
    trigger::UserTriggerTarget,
};

#[derive(Debug, Clone, Id, EnumVariants)]
pub enum BlockAutomation {
    BathroomDehumidifier,
    BedroomDehumidifier,
}

impl Rule for BlockAutomation {
    fn evaluate(&self, ctx: &RuleEvaluationContext) -> anyhow::Result<RuleResult> {
        let Some(night_time) = t!(22:00 - 9:00).active() else {
            return Ok(RuleResult::Skip);
        };

        let trigger = match self {
            BlockAutomation::BathroomDehumidifier => {
                ctx.latest_trigger(UserTriggerTarget::Homekit(HomekitCommandTarget::DehumidifierPower))
            }
            BlockAutomation::BedroomDehumidifier => {
                ctx.latest_trigger(UserTriggerTarget::Homekit(HomekitCommandTarget::BedroomDehumidifierFanSpeed))
            }
        };

        //User-trigger after night time starts -> user really wants to override
        if let Some(trigger) = trigger
            && night_time.contains(&trigger.timestamp)
        {
            let result = UserTriggerAction::new(trigger.target().clone()).evaluate(ctx);
            if matches!(result, Ok(RuleResult::ExecuteTrigger(..))) {
                return result;
            }
        }

        match self {
            BlockAutomation::BathroomDehumidifier => {
                FollowDefaultSetting::new(crate::command::CommandTarget::SetPower {
                    device: PowerToggle::Dehumidifier,
                })
            }
            BlockAutomation::BedroomDehumidifier => {
                FollowDefaultSetting::new(crate::command::CommandTarget::ControlFan {
                    device: Fan::BedroomDehumidifier,
                })
            }
        }
        .evaluate(ctx)
    }
}
