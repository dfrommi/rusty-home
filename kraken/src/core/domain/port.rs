#![allow(async_fn_in_trait)]

use api::command::Command;
use tokio::sync::mpsc;

use crate::core::IncomingData;

pub trait CommandExecutor {
    //Returns true if command was executed
    async fn execute_command(&self, command: &Command) -> anyhow::Result<bool>;
}

pub trait IncomingDataProcessor {
    async fn process(&mut self, sender: mpsc::Sender<IncomingData>) -> anyhow::Result<()>;
}
