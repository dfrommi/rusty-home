pub mod db;
mod homeassistant;
mod tasmota;
mod z2m;

use infrastructure::TraceContext;

use crate::command::{Command, CommandClient, CommandExecution};

pub use homeassistant::HomeAssistantCommandExecutor;
pub use tasmota::TasmotaCommandExecutor;
pub use z2m::Z2mCommandExecutor;

pub trait CommandExecutor {
    //Returns true if command was executed
    async fn execute_command(&self, command: &Command) -> anyhow::Result<bool>;
}

#[tracing::instrument(skip_all, fields(command = ?cmd.command))]
async fn process_command(cmd: CommandExecution, command_client: &CommandClient, executor: &impl CommandExecutor) {
    TraceContext::continue_from(&cmd.correlation_id);

    let res = executor.execute_command(&cmd.command).await;

    handle_execution_result(cmd.id, res, command_client).await;
}

async fn handle_execution_result(command_id: i64, res: anyhow::Result<bool>, command_client: &CommandClient) {
    let set_state_res = match res {
        Ok(true) => command_client.set_command_state_success(command_id).await,
        Ok(false) => Ok(()),
        Err(e) => {
            tracing::error!("Command {} failed: {:?}", command_id, e);
            command_client.set_command_state_error(command_id, &e.to_string()).await
        }
    };

    if let Err(e) = set_state_res {
        tracing::error!("Error setting command state for {}: {}", command_id, e);
    }
}
