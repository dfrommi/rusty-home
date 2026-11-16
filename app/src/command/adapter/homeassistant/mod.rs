use infrastructure::HttpClientConfig;
use reqwest_middleware::ClientWithMiddleware;

use super::metrics::*;
use crate::command::{LightSignal, PushNotification, PushNotifyTarget, PushRecipient, WashingMachineState};
use crate::core::unit::{FanAirflow, FanSpeed};
use crate::t;
use serde_json::json;

const PUSH_TAG_WINDOW_OPENED: &str = "window_opened";
const PUSH_TAG_WASHING_MACHINE: &str = "washing_machine";

pub struct HomeAssistantCommandExecutor {
    client: HaHttpClient,
}

impl HomeAssistantCommandExecutor {
    #[allow(clippy::expect_used)]
    pub fn new(url: &str, token: &str) -> Self {
        let http_client = HaHttpClient::new(url, token).expect("Error initializing Home Assistant REST client");

        Self { client: http_client }
    }

    pub async fn dismiss_push_notification(&self, target: PushNotifyTarget) -> anyhow::Result<()> {
        let service = super::push_recipient(target.recipient());

        let tag = match target {
            PushNotifyTarget::WindowOpened(_) => PUSH_TAG_WINDOW_OPENED,
            PushNotifyTarget::WashingMachine(_) => PUSH_TAG_WASHING_MACHINE,
        };

        self.client
            .call_service(
                "notify",
                service,
                json!({
                    "message": "clear_notification",
                    "data": { "tag": tag }
                }),
            )
            .await?;

        record_executed(service);
        Ok(())
    }

    pub async fn show_push_notification(
        &self,
        recipient: PushRecipient,
        notification: PushNotification,
    ) -> anyhow::Result<()> {
        let service = super::push_recipient(recipient);

        let payload = match notification {
            PushNotification::WindowOpened => json!({
                "title": "Fenster offen",
                "message": "Mindestens ein Fenster ist offen",
                "data": { "tag": PUSH_TAG_WINDOW_OPENED }
            }),
            PushNotification::WashingMachine(state) => washing_machine_notification_payload(&state),
        };

        self.client.call_service("notify", service, payload).await?;
        record_executed(service);
        Ok(())
    }

    pub async fn set_power(&self, id: &str, power_on: bool) -> anyhow::Result<()> {
        let service = if power_on { "turn_on" } else { "turn_off" };
        self.client
            .call_service("switch", service, json!({ "entity_id": [id] }))
            .await?;
        record_executed(id);
        Ok(())
    }

    pub async fn set_light_notification(&self, id: &str, _signal: LightSignal, on: bool) -> anyhow::Result<()> {
        let service = if on { "turn_on" } else { "turn_off" };
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

fn record_executed(id: &str) {
    CommandMetric::Executed {
        device_id: id.to_string(),
        system: CommandTargetSystem::HomeAssistant,
    }
    .record();
}

fn washing_machine_notification_payload(state: &WashingMachineState) -> serde_json::Value {
    match state {
        WashingMachineState::Done => json!({
            "title": "Wäsche fertig",
            "message": "Die Wäsche ist fertig",
            "data": {
                "tag": PUSH_TAG_WASHING_MACHINE,
                "live_update": true,
                "progress": 100,
                "progress_max": 100,
                "progress_bar_direction": "increasing",
                "notification_icon": "mdi:washing-machine",
            }
        }),
        WashingMachineState::InProgress {
            started_at,
            estimated_end,
        } => {
            let now = t!(now);
            let started_at = *started_at;
            let estimated_end = (*estimated_end)
                .max(now + t!(2 minutes))
                .max(started_at + t!(100 seconds));
            let now = now.unix_seconds();
            let started_at = started_at.unix_seconds();
            let estimated_end = estimated_end.unix_seconds();
            let range = (estimated_end - started_at).max(100);
            json!({
                "title": "Wäsche fertig",
                "message": "Die Wäsche ist fertig",
                "data": {
                    "tag": PUSH_TAG_WASHING_MACHINE,
                    "live_update": true,
                    "silent": true,
                    "chronometer": true,
                    "when_relative": false,
                    "when": estimated_end,
                    "progress": (now - started_at).clamp(0, range),
                    "progress_max": range,
                    "progress_bar_direction": "increasing",
                    "notification_icon": "mdi:washing-machine",
                }
            })
        }
    }
}

fn philips_air_purifier_preset(speed: &FanSpeed) -> &'static str {
    match speed {
        FanSpeed::Low => "speed_1",
        FanSpeed::Medium => "speed_2",
        FanSpeed::High => "speed_3",
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

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::core::time::{DateTime, Duration};
    use serde_json::json;
    #[test]
    fn maps_philips_air_purifier_command_speed_presets() {
        assert_eq!(philips_air_purifier_preset(&FanSpeed::Low), "speed_1");
        assert_eq!(philips_air_purifier_preset(&FanSpeed::Medium), "speed_2");
        assert_eq!(philips_air_purifier_preset(&FanSpeed::High), "speed_3");
    }

    #[tokio::test]
    async fn washing_machine_progress_reflects_elapsed_time_before_estimated_end() {
        let now = DateTime::from_static_iso("2025-01-01T12:00:00Z");
        let payload = now
            .eval_timeshifted(async {
                washing_machine_notification_payload(&WashingMachineState::InProgress {
                    started_at: now - Duration::minutes(10),
                    estimated_end: now + Duration::minutes(15),
                })
            })
            .await;

        assert_eq!(payload["data"]["progress"], json!(600));
        assert_eq!(payload["data"]["progress_max"], json!(1500));
    }
    #[tokio::test]
    async fn washing_machine_progress_uses_a_safe_future_end_for_a_past_estimate() {
        let now = DateTime::from_static_iso("2025-01-01T12:00:00Z");
        let payload = now
            .eval_timeshifted(async {
                washing_machine_notification_payload(&WashingMachineState::InProgress {
                    started_at: now - Duration::minutes(10),
                    estimated_end: now - Duration::minutes(1),
                })
            })
            .await;

        assert_eq!(payload["data"]["when"], json!((now + Duration::minutes(2)).unix_seconds()));
        assert_eq!(payload["data"]["progress"], json!(600));
        assert_eq!(payload["data"]["progress_max"], json!(720));
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
            .set_light_notification("light.hue_go", LightSignal::WindowOpened, true)
            .await
            .unwrap();
        executor
            .set_light_notification("light.hue_go", LightSignal::WindowOpened, false)
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
                .set_light_notification("light.hue_go", LightSignal::WindowOpened, true)
                .await
                .is_err()
        );
    }
}
