use api::command::CommandSource;
use chrono::{DateTime, Utc};
use support::{ext::ResultExt, t};

use crate::{home_api, thing::planning::action::Action};

pub trait ActionPlannerExt {
    fn command_source_start(&self) -> CommandSource;
    fn command_source_stop(&self) -> CommandSource;
    async fn preconditions_fulfilled_or_default(&self) -> bool;
    async fn is_running_or_scheduled_or_default(&self) -> bool;

    async fn was_started_since(&self, since: DateTime<Utc>) -> bool;
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum CommandSourceType {
    Start,
    Stop,
}

impl<A: Action> ActionPlannerExt for A {
    fn command_source_start(&self) -> CommandSource {
        CommandSource::System(format!("planning:{}:start", self))
    }

    fn command_source_stop(&self) -> CommandSource {
        CommandSource::System(format!("planning:{}:stop", self))
    }

    async fn preconditions_fulfilled_or_default(&self) -> bool {
        self.preconditions_fulfilled().await.unwrap_or_warn(
            false,
            format!(
                "Error checking preconditions of {}, assuming not fulfilled",
                self
            )
            .as_str(),
        )
    }

    async fn is_running_or_scheduled_or_default(&self) -> bool {
        let last_command_type = get_last_command_type_since(self, t!(1 minutes ago)).await;

        match last_command_type {
            Some(CommandSourceType::Start) => true,
            Some(CommandSourceType::Stop) => false,
            None => self.is_running().await.unwrap_or_warn(
                false,
                format!("Error checking running state of action {}", self).as_str(),
            ),
        }
    }

    async fn was_started_since(&self, since: DateTime<Utc>) -> bool {
        get_last_command_type_since(self, since)
            .await
            .map(|t| t == CommandSourceType::Start)
            .unwrap_or(false)
    }
}

async fn get_last_command_type_since(
    action: &impl Action,
    since: DateTime<Utc>,
) -> Option<CommandSourceType> {
    let last_source = if let Some(target) = action.controls_target() {
        home_api()
            .get_latest_command_source_since(target, since)
            .await
            .unwrap_or_warn(
                None,
                format!("Error getting last command type of {}", action).as_str(),
            )
    } else {
        None
    };

    if let Some(last_source) = last_source {
        if last_source == action.command_source_start() {
            return Some(CommandSourceType::Start);
        } else if last_source == action.command_source_stop() {
            return Some(CommandSourceType::Stop);
        }
    }

    None
}
