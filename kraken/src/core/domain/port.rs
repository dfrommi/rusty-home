#![allow(async_fn_in_trait)]

use anyhow::Result;
use api::{
    command::{Command, CommandExecution},
    state::ChannelValue,
    trigger::UserTrigger,
};
use support::time::DateTime;
use tokio::sync::mpsc;

use crate::core::IncomingData;

pub trait CommandExecutor {
    //Returns true if command was executed
    async fn execute_command(&self, command: &Command) -> anyhow::Result<bool>;
}

pub trait IncomingDataProcessor {
    async fn process(&mut self, sender: mpsc::Sender<IncomingData>) -> anyhow::Result<()>;
}

pub trait CommandRepository {
    async fn get_command_for_processing(&self) -> Result<Option<CommandExecution>>;
    async fn set_command_state_success(&self, command_id: i64) -> Result<()>;
    async fn set_command_state_error(&self, command_id: i64, error_message: &str) -> Result<()>;
}

pub trait StateStorage {
    async fn add_state(&self, value: &ChannelValue, timestamp: &DateTime) -> anyhow::Result<()>;
}

pub trait UserTriggerStorage {
    async fn add_user_trigger(&self, trigger: UserTrigger) -> anyhow::Result<()>;
}
