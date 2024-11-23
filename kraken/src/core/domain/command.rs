use super::port::{CommandExecutor, CommandRepository, NewCommandAvailableTrigger};

use anyhow::Result;

pub async fn execute_commands(
    repo: &impl CommandRepository,
    executor: &impl CommandExecutor,
    new_command_available: &mut impl NewCommandAvailableTrigger,
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

                let res = executor.execute_command(&cmd.command).await;
                //TODO loop over executors and check bool result

                handle_execution_result(cmd.id, res, repo).await;
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

async fn handle_execution_result(
    command_id: i64,
    res: Result<bool>,
    repo: &impl CommandRepository,
) {
    let set_state_res = match res {
        Ok(true) => repo.set_command_state_success(command_id).await,
        Ok(false) => Ok(()),
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
