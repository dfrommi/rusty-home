use infrastructure::HttpClientConfig;
use reqwest_middleware::ClientWithMiddleware;

use super::metrics::*;
use crate::command::{NotificationKind, NotificationOperation};
use crate::core::unit::{FanAirflow, FanSpeed};
use serde_json::json;

pub struct HomeAssistantCommandExecutor {
    client: HaHttpClient,
}

impl HomeAssistantCommandExecutor {
    #[allow(clippy::expect_used)]
    pub fn new(url: &str, token: &str) -> Self {
        let http_client = HaHttpClient::new(url, token).expect("Error initializing Home Assistant REST client");

        Self { client: http_client }
    }

    pub async fn notify_phone(
        &self,
        service: &str,
        notification: NotificationKind,
        operation: NotificationOperation,
    ) -> anyhow::Result<()> {
        match operation {
            NotificationOperation::Show => {
                let (title, message, tag) = notification_content(notification);
                self.client
                    .call_service(
                        "notify",
                        service,
                        json!({
                            "title": title,
                            "message": message,
                            "data": { "tag": tag }
                        }),
                    )
                    .await?;
            }
            NotificationOperation::Dismiss => {
                self.client
                    .call_service(
                        "notify",
                        service,
                        json!({
                            "message": "clear_notification",
                            "data": { "tag": notification_tag(notification) }
                        }),
                    )
                    .await?;
            }
        }
        record_executed(service);
        Ok(())
    }

    pub async fn notify_light(
        &self,
        id: &str,
        _notification: NotificationKind,
        operation: NotificationOperation,
    ) -> anyhow::Result<()> {
        let service = match operation {
            NotificationOperation::Show => "turn_on",
            NotificationOperation::Dismiss => "turn_off",
        };
        self.client
            .call_service("light", service, json!({ "entity_id": [id] }))
            .await?;
        record_executed(id);
        Ok(())
    }

    pub async fn set_comfee_fan_speed(
        &self,
        humidifier_id: &str,
        fan_id: &str,
        airflow: &FanAirflow,
    ) -> anyhow::Result<()> {
        match airflow {
            FanAirflow::Off => {
                self.client
                    .call_service(
                        "humidifier",
                        "turn_off",
                        json!({
                            "entity_id": vec![humidifier_id.to_string()]
                        }),
                    )
                    .await?
            }
            FanAirflow::Forward(fan_speed) => {
                let fan_preset = match fan_speed {
                    FanSpeed::Low => "Low",
                    FanSpeed::Medium => "Medium",
                    FanSpeed::High => "High",
                };

                self.client
                    .call_service(
                        "humidifier",
                        "turn_on",
                        json!({
                            "entity_id": vec![humidifier_id.to_string()]
                        }),
                    )
                    .await?;
                record_executed(humidifier_id);

                self.client
                    .call_service(
                        "fan",
                        "set_preset_mode",
                        json!({
                            "entity_id": vec![fan_id.to_string()],
                            "preset_mode": fan_preset
                        }),
                    )
                    .await?;
                record_executed(fan_id);
            }
        };

        Ok(())
    }

    pub async fn set_philips_air_purifier_speed(&self, id: &str, airflow: &FanAirflow) -> anyhow::Result<()> {
        match airflow {
            FanAirflow::Off => {
                self.client
                    .call_service(
                        "fan",
                        "turn_off",
                        json!({
                            "entity_id": vec![id.to_string()]
                        }),
                    )
                    .await?;
            }
            FanAirflow::Forward(fan_speed) => {
                self.client
                    .call_service(
                        "fan",
                        "turn_on",
                        json!({
                            "entity_id": vec![id.to_string()],
                            "preset_mode": philips_air_purifier_preset(fan_speed)
                        }),
                    )
                    .await?;
            }
        }

        record_executed(id);
        Ok(())
    }
}

fn notification_content(notification: NotificationKind) -> (&'static str, &'static str, &'static str) {
    match notification {
        NotificationKind::WindowOpened => ("Fenster offen", "Mindestens ein Fenster ist offen", "window_opened"),
    }
}

fn notification_tag(notification: NotificationKind) -> &'static str {
    match notification {
        NotificationKind::WindowOpened => "window_opened",
    }
}

fn record_executed(id: &str) {
    CommandMetric::Executed {
        device_id: id.to_string(),
        system: CommandTargetSystem::HomeAssistant,
    }
    .record();
}

fn philips_air_purifier_preset(speed: &FanSpeed) -> &'static str {
    match speed {
        FanSpeed::Low => "speed_1",
        FanSpeed::Medium => "speed_2",
        FanSpeed::High => "speed_3",
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn maps_philips_air_purifier_command_speed_presets() {
        assert_eq!(philips_air_purifier_preset(&FanSpeed::Low), "speed_1");
        assert_eq!(philips_air_purifier_preset(&FanSpeed::Medium), "speed_2");
        assert_eq!(philips_air_purifier_preset(&FanSpeed::High), "speed_3");
    }

    #[test]
    fn window_open_notification_keeps_its_mobile_presentation() {
        assert_eq!(
            notification_content(NotificationKind::WindowOpened),
            ("Fenster offen", "Mindestens ein Fenster ist offen", "window_opened")
        );
        assert_eq!(notification_tag(NotificationKind::WindowOpened), "window_opened");
    }

    #[tokio::test]
    async fn sends_phone_notification_to_provided_home_assistant_service() {
        let mut server = mockito::Server::new_async().await;
        let _mock = server
            .mock("POST", "/api/services/notify/mobile_app_jarvis")
            .with_status(200)
            .expect(1)
            .create_async()
            .await;
        let executor = HomeAssistantCommandExecutor::new(&server.url(), "token");

        executor
            .notify_phone("mobile_app_jarvis", NotificationKind::WindowOpened, NotificationOperation::Show)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn sends_indicator_show_and_dismiss_to_provided_light() {
        let mut server = mockito::Server::new_async().await;
        let _on = server
            .mock("POST", "/api/services/light/turn_on")
            .with_status(200)
            .expect(1)
            .create_async()
            .await;
        let _off = server
            .mock("POST", "/api/services/light/turn_off")
            .with_status(200)
            .expect(1)
            .create_async()
            .await;
        let executor = HomeAssistantCommandExecutor::new(&server.url(), "token");
        executor
            .notify_light("light.hue_go", NotificationKind::WindowOpened, NotificationOperation::Show)
            .await
            .unwrap();
        executor
            .notify_light("light.hue_go", NotificationKind::WindowOpened, NotificationOperation::Dismiss)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn home_assistant_service_error_is_returned() {
        let mut server = mockito::Server::new_async().await;
        let _mock = server
            .mock("POST", "/api/services/light/turn_on")
            .with_status(500)
            .expect(1)
            .create_async()
            .await;
        let executor = HomeAssistantCommandExecutor::new(&server.url(), "token");

        assert!(
            executor
                .notify_light("light.hue_go", NotificationKind::WindowOpened, NotificationOperation::Show)
                .await
                .is_err()
        );
    }
}

#[derive(Debug, Clone)]
pub struct HaHttpClient {
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
    pub async fn call_service(
        &self,
        domain: &str,
        service: &str,
        service_data: serde_json::Value,
    ) -> anyhow::Result<()> {
        let url = format!("{}/api/services/{}/{}", self.base_url, domain, service);

        tracing::info!("Calling HA service {}: {:?}", url, serde_json::to_string(&service_data)?);

        let response = self.client.post(url).json(&service_data).send().await?;
        let status = response.status();
        let body = response.text().await?;
        tracing::info!("Response: {} - {}", status, body);

        if !status.is_success() {
            anyhow::bail!("Home Assistant service returned HTTP status {status}");
        }

        Ok(())
    }
}
