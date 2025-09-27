use crate::core::{HomeApi, app_event::CommandAddedEvent};

use crate::Infrastructure;
use crate::home::command::{Command, CommandExecution};
use anyhow::Result;
use infrastructure::TraceContext;
use tokio::sync::broadcast::Receiver;

pub trait CommandExecutor {
    //Returns true if command was executed
    async fn execute_command(&self, command: &Command) -> anyhow::Result<bool>;
}

pub fn keep_command_executor_running<A: CommandExecutor, B: CommandExecutor, C: CommandExecutor>(
    infrastructure: &Infrastructure,
    first: A,
    second: B,
    third: C,
) -> impl Future<Output = ()> + use<A, B, C> {
    let command_repo = infrastructure.api.clone();
    let new_cmd_available = infrastructure.event_listener.new_command_added_listener();

    let cmd_executor = MultiCommandExecutor { first, second, third };

    async move {
        process_command_executor(&command_repo, &cmd_executor, new_cmd_available).await;
    }
}

async fn process_command_executor(
    api: &HomeApi,
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

        let command = api.get_command_for_processing().await;

        match command {
            Ok(Some(cmd)) => {
                got_cmd = true;
                process_command(cmd, api, executor).await;
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
async fn process_command(cmd: CommandExecution, api: &HomeApi, executor: &impl CommandExecutor) {
    TraceContext::continue_from(&cmd.correlation_id);

    let res = executor.execute_command(&cmd.command).await;

    handle_execution_result(cmd.id, res, api).await;
}

async fn handle_execution_result(command_id: i64, res: Result<bool>, api: &HomeApi) {
    let set_state_res = match res {
        Ok(true) => api.set_command_state_success(command_id).await,
        Ok(false) => {
            tracing::error!("No command executor configured for command {}", command_id);
            api.set_command_state_error(command_id, "No command executor configured")
                .await
        }
        Err(e) => {
            tracing::error!("Command {} failed: {:?}", command_id, e);
            api.set_command_state_error(command_id, &e.to_string()).await
        }
    };

    if let Err(e) = set_state_res {
        tracing::error!("Error setting command state for {}: {}", command_id, e);
    }
}

struct MultiCommandExecutor<A, B, C>
where
    A: CommandExecutor,
    B: CommandExecutor,
    C: CommandExecutor,
{
    first: A,
    second: B,
    third: C,
}

impl<A, B, C> CommandExecutor for MultiCommandExecutor<A, B, C>
where
    A: CommandExecutor,
    B: CommandExecutor,
    C: CommandExecutor,
{
    async fn execute_command(&self, command: &Command) -> anyhow::Result<bool> {
        if self.first.execute_command(command).await? {
            return Ok(true);
        }
        if self.second.execute_command(command).await? {
            return Ok(true);
        }
        if self.third.execute_command(command).await? {
            return Ok(true);
        }

        anyhow::bail!("No command executor configured for command {:?}", command)
    }
}
