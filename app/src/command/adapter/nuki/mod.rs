mod config;

use super::metrics::*;
use crate::command::adapter::CommandExecutor;
use crate::command::{Command, CommandTarget, Lock};
use infrastructure::HttpClientConfig;
use reqwest_middleware::ClientWithMiddleware;

#[derive(Debug, Clone)]
enum NukiCommandTarget {
    Opener(&'static str),
}

pub struct NukiCommandExecutor {
    client: ClientWithMiddleware,
    bridge_url: String,
    token: String,
    config: Vec<(CommandTarget, NukiCommandTarget)>,
}

impl NukiCommandExecutor {
    #[allow(clippy::expect_used)]
    pub fn new(bridge_url: &str, token: &str) -> Self {
        let client = HttpClientConfig::new(None)
            .new_tracing_client()
            .expect("Error initializing HTTP client for Nuki Bridge");

        let config = config::default_nuki_command_config();

        Self {
            client,
            bridge_url: bridge_url.to_owned(),
            token: token.to_owned(),
            config,
        }
    }
}

impl CommandExecutor for NukiCommandExecutor {
    #[tracing::instrument(name = "execute_command NUKI", ret, skip(self))]
    async fn execute_command(&self, command: &Command) -> anyhow::Result<bool> {
        let cmd_target: CommandTarget = command.into();
        let nuki_target = self
            .config
            .iter()
            .find_map(|(cmd, nuki)| if cmd == &cmd_target { Some(nuki) } else { None });

        let Some(NukiCommandTarget::Opener(nuki_id)) = nuki_target else {
            return Ok(false);
        };

        // Confirm command is OpenDoor (compile-time safety)
        let Command::OpenDoor {
            device: Lock::BuildingEntrance,
        } = command
        else {
            anyhow::bail!("Mismatch between command and Nuki target {:?}", nuki_target);
        };

        let url = format!(
            "{}/lockAction?nukiId={}&deviceType=2&action=3&token={}",
            self.bridge_url, nuki_id, self.token
        );

        let response = self.client.get(&url).send().await?;
        let body: serde_json::Value = response.json().await?;

        if body.get("success").and_then(|v| v.as_bool()) == Some(true) {
            CommandMetric::Executed {
                device_id: (*nuki_id).to_string(),
                system: CommandTargetSystem::Nuki,
            }
            .record();
            Ok(true)
        } else {
            anyhow::bail!("Nuki bridge returned non-success: {:?}", body);
        }
    }
}
