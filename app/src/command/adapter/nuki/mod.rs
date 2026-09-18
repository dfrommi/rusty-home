use super::metrics::{CommandMetric, CommandTargetSystem};
use infrastructure::HttpClientConfig;
use reqwest_middleware::ClientWithMiddleware;

pub struct NukiCommandExecutor {
    client: ClientWithMiddleware,
    bridge_url: String,
    token: String,
}

impl NukiCommandExecutor {
    #[allow(clippy::expect_used)]
    pub fn new(bridge_url: &str, token: &str) -> Self {
        let client = HttpClientConfig::new(None)
            .new_tracing_client()
            .expect("Error initializing HTTP client for Nuki Bridge");

        Self {
            client,
            bridge_url: bridge_url.to_owned(),
            token: token.to_owned(),
        }
    }

    pub async fn open_door(&self, nuki_id: &str) -> anyhow::Result<()> {
        let url = format!(
            "{}/lockAction?nukiId={}&deviceType=2&action=3&token={}",
            self.bridge_url, nuki_id, self.token
        );

        let response = self.client.get(&url).send().await?;
        let body: serde_json::Value = response.json().await?;

        if body.get("success").and_then(|v| v.as_bool()) == Some(true) {
            CommandMetric::Executed {
                device_id: nuki_id.to_string(),
                system: CommandTargetSystem::Nuki,
            }
            .record();
            Ok(())
        } else {
            anyhow::bail!("Nuki bridge returned non-success: {:?}", body);
        }
    }
}
