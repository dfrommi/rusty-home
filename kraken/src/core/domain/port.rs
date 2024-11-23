#![allow(async_fn_in_trait)]

use anyhow::Result;
use api::{
    command::{Command, CommandExecution},
    state::ChannelValue,
};
use support::{time::DateTime, DataPoint};

pub trait CommandExecutor {
    //Returns true if command was executed
    async fn execute_command(&self, command: &Command) -> anyhow::Result<bool>;
}

pub trait StateCollector {
    async fn get_current_state(&self) -> anyhow::Result<Vec<DataPoint<ChannelValue>>>;
    async fn recv(&mut self) -> anyhow::Result<DataPoint<ChannelValue>>;
}

pub trait CommandRepository {
    async fn get_command_for_processing(&self) -> Result<Option<CommandExecution<Command>>>;
    async fn set_command_state_success(&self, command_id: i64) -> Result<()>;
    async fn set_command_state_error(&self, command_id: i64, error_message: &str) -> Result<()>;
}

pub trait NewCommandAvailableTrigger {
    async fn recv(&mut self);
}

pub trait StateStorage {
    async fn add_state(&self, value: &ChannelValue, timestamp: &DateTime) -> anyhow::Result<()>;
}
