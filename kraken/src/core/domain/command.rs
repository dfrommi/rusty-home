use crate::core::event::CommandAddedEvent;

use super::port::{CommandExecutor, CommandRepository};

use anyhow::Result;
use api::command::CommandExecution;
use infrastructure::TraceContext;
use tokio::sync::broadcast::Receiver;

pub async fn execute_commands(
    repo: &impl CommandRepository,
    executor: &impl CommandExecutor,
    mut new_command_available: Receiver<CommandAddedEvent>,
) {
    let mut timer = tokio::time::interval(std::time::Duration::from_secs(15));
    let mut got_cmd = false;

    loop {
        //Busy loop if command was found to process as much as possible
        if !got_cmd {
            tokio::select! {
                _ = new_command_available.recv() => {},
                _ = timer.tick() => {},
            };
        }

        let command = repo.get_command_for_processing().await;

        match command {
            Ok(Some(cmd)) => {
                got_cmd = true;
                process_command(cmd, repo, executor).await;
            }
            Ok(None) => {
                got_cmd = false;
            }
            Err(e) => {
                tracing::error!("Error getting pending commands: {:?}", e);
                got_cmd = false;
            }
        }
    }
}

#[tracing::instrument(skip_all, fields(command = ?cmd.command))]
async fn process_command(
    cmd: CommandExecution,
    repo: &impl CommandRepository,
    executor: &impl CommandExecutor,
) {
    TraceContext::continue_from(&cmd.correlation_id);

    let res = executor.execute_command(&cmd.command).await;

    handle_execution_result(cmd.id, res, repo).await;
}

async fn handle_execution_result(
    command_id: i64,
    res: Result<bool>,
    repo: &impl CommandRepository,
) {
    let set_state_res = match res {
        Ok(true) => repo.set_command_state_success(command_id).await,
        Ok(false) => {
            tracing::error!("No command executor configured for command {}", command_id);
            repo.set_command_state_error(command_id, "No command executor configured")
                .await
        }
        Err(e) => {
            tracing::error!("Command {} failed: {:?}", command_id, e);
            repo.set_command_state_error(command_id, &e.to_string())
                .await
        }
    };

    if let Err(e) = set_state_res {
        tracing::error!("Error setting command state for {}: {}", command_id, e);
    }
}
