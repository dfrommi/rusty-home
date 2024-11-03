use api::command::CommandSource;
use support::{ext::ResultExt, t};

use crate::{home_api, thing::planning::action::Action};

pub trait ActionPlannerExt {
    fn command_source_start(&self) -> CommandSource;
    fn command_source_stop(&self) -> CommandSource;
    async fn preconditions_fulfilled_or_log_error(&self) -> bool;
    async fn is_running_or_scheduled(&self) -> bool;
}

impl<A: Action> ActionPlannerExt for A {
    fn command_source_start(&self) -> CommandSource {
        CommandSource::System(format!("planning:{}:start", self))
    }

    fn command_source_stop(&self) -> CommandSource {
        CommandSource::System(format!("planning:{}:stop", self))
    }

    async fn preconditions_fulfilled_or_log_error(&self) -> bool {
        self.preconditions_fulfilled().await.unwrap_or_else(|e| {
            tracing::warn!(
                "Error checking preconditions of action {:?}, assuming not fulfilled: {:?}",
                self,
                e
            );
            false
        })
    }

    async fn is_running_or_scheduled(&self) -> bool {
        let last_command = if let Some(target) = self.controls_target() {
            home_api()
                .get_latest_command_since(target, t!(1 minutes ago))
                .await
                .unwrap_or_warn(
                    None,
                    format!("Error getting last command of {}", self).as_str(),
                )
        } else {
            None
        };

        if let Some(last_command) = last_command {
            if last_command.source == self.command_source_start() {
                return true;
            } else if last_command.source == self.command_source_stop() {
                return false;
            }
        }

        self.is_running().await.unwrap_or_else(|e| {
            tracing::warn!(
                "Error checking running state of action {:?}, assuming not running: {:?}",
                self,
                e
            );
            false
        })
    }
}
