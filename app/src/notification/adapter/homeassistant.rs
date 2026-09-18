use infrastructure::HttpClientConfig;
use reqwest_middleware::ClientWithMiddleware;
use serde_json::json;

use crate::command::{Notification, NotificationRecipient};
use crate::notification::NotificationId;
use crate::observability::system_metric_increment;

pub struct HomeAssistantNotificationExecutor {
    client: HaHttpClient,
}

impl HomeAssistantNotificationExecutor {
    #[allow(clippy::expect_used)]
    pub fn new(url: &str, token: &str) -> Self {
        let http_client = HaHttpClient::new(url, token).expect("Error initializing Home Assistant REST client");

        Self { client: http_client }
    }

    pub async fn notify(&self, recipient: &NotificationRecipient, notification: &Notification) -> anyhow::Result<()> {
        let mobile_id = mobile_id(recipient);
        let (title, message, tag) = match notification {
            Notification::WindowOpened => ("Fenster offen", "Mindestens ein Fenster ist offen", "window_opened"),
        };

        self.client
            .call_service(
                mobile_id,
                json!({
                    "title": title,
                    "message": message,
                    "data": {
                        "tag": tag
                    }
                }),
            )
            .await?;
        record_executed(mobile_id);

        Ok(())
    }

    pub async fn dismiss(&self, recipient: &NotificationRecipient, notification: NotificationId) -> anyhow::Result<()> {
        let mobile_id = mobile_id(recipient);
        let tag = match notification {
            NotificationId::WindowOpened => "window_opened",
        };

        self.client
            .call_service(
                mobile_id,
                json!({
                    "message": "clear_notification",
                    "data": {
                        "tag": tag
                    }
                }),
            )
            .await?;
        record_executed(mobile_id);

        Ok(())
    }
}

fn mobile_id(recipient: &NotificationRecipient) -> &'static str {
    match recipient {
        NotificationRecipient::Dennis => "mobile_app_jarvis",
        NotificationRecipient::Sabine => "mobile_app_simi_2",
    }
}

fn record_executed(id: &str) {
    system_metric_increment("command_executed", &[("device_id", id), ("system", "HA")]);
}

#[derive(Debug, Clone)]
struct HaHttpClient {
    client: ClientWithMiddleware,
    base_url: String,
}

impl HaHttpClient {
    pub fn new(url: &str, token: &str) -> anyhow::Result<Self> {
        let client = HttpClientConfig::new(Some(token.to_owned())).new_tracing_client()?;

        Ok(Self {
            client,
            base_url: url.to_owned(),
        })
    }

    #[tracing::instrument(skip(self))]
    pub async fn call_service(&self, service: &str, service_data: serde_json::Value) -> anyhow::Result<()> {
        let url = format!("{}/api/services/notify/{}", self.base_url, service);

        tracing::info!(
            "Calling HA notification service {}: {:?}",
            url,
            serde_json::to_string(&service_data)?
        );

        let response = self.client.post(url).json(&service_data).send().await?;
        let status = response.status();
        let body = response.text().await?;
        tracing::info!("Response: {} - {}", status, body);

        if !status.is_success() {
            anyhow::bail!("Home Assistant notification service returned HTTP status {status}");
        }

        Ok(())
    }
}
