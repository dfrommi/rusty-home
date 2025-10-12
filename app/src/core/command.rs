use crate::core::{HomeApi, app_event::CommandAddedEvent};

use crate::Infrastructure;
use crate::home::command::CommandExecution;

pub struct CommandDispatcher {
    api: HomeApi,
    new_command_available_rx: tokio::sync::broadcast::Receiver<CommandAddedEvent>,
    command_pending_tx: tokio::sync::broadcast::Sender<CommandExecution>,
}

impl CommandDispatcher {
    pub fn new(infrastructure: &Infrastructure) -> CommandDispatcher {
        let api = infrastructure.api.clone();
        let new_command_available_rx = infrastructure.event_listener.new_command_added_listener();

        Self {
            api,
            new_command_available_rx,
            command_pending_tx: tokio::sync::broadcast::channel(32).0,
        }
    }

    pub fn subscribe(&self) -> tokio::sync::broadcast::Receiver<CommandExecution> {
        self.command_pending_tx.subscribe()
    }

    pub async fn dispatch(&mut self) {
        let mut timer = tokio::time::interval(std::time::Duration::from_secs(15));
        let mut got_cmd = false;

        loop {
            //Busy loop if command was found to process as much as possible
            if !got_cmd {
                tokio::select! {
                    _ = self.new_command_available_rx.recv() => {},
                    _ = timer.tick() => {},
                };
            }

            let command = self.api.get_command_for_processing().await;

            match command {
                Ok(Some(cmd)) => {
                    got_cmd = true;
                    self.command_pending_tx.send(cmd).ok();
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
}
