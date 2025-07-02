#![allow(async_fn_in_trait)]

use api::command::Command;

pub trait CommandExecutor {
    //Returns true if command was executed
    async fn execute_command(&self, command: &Command) -> anyhow::Result<bool>;
}
