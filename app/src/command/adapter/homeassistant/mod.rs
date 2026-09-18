use infrastructure::HttpClientConfig;
use reqwest_middleware::ClientWithMiddleware;

use super::metrics::*;
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

    pub async fn set_light_power(&self, id: &str, power_on: bool) -> anyhow::Result<()> {
        let service = if power_on { "turn_on" } else { "turn_off" };
        self.client
            .call_service(
                "light",
                service,
                json!({
                    "entity_id": vec![id.to_string()],
                }),
            )
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

    pub async fn notify_window_opened(&self, mobile_id: &str) -> anyhow::Result<()> {
        self.client
            .call_service(
                "notify",
                mobile_id,
                json!({
                    "title": "Fenster offen",
                    "message": "Mindestens ein Fenster ist offen",
                    "data": {
                        "tag": "window_opened"
                    }
                }),
            )
            .await?;

        record_executed(mobile_id);

        Ok(())
    }

    pub async fn dismiss_window_opened_notification(&self, mobile_id: &str) -> anyhow::Result<()> {
        self.client
            .call_service(
                "notify",
                mobile_id,
                json!({
                    "message": "clear_notification",
                    "data": {
                        "tag": "window_opened"
                    }
                }),
            )
            .await?;

        record_executed(mobile_id);

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

fn philips_air_purifier_preset(speed: &FanSpeed) -> &'static str {
    match speed {
        FanSpeed::Low => "speed_1",
        FanSpeed::Medium => "speed_2",
        FanSpeed::High => "speed_3",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_philips_air_purifier_command_speed_presets() {
        assert_eq!(philips_air_purifier_preset(&FanSpeed::Low), "speed_1");
        assert_eq!(philips_air_purifier_preset(&FanSpeed::Medium), "speed_2");
        assert_eq!(philips_air_purifier_preset(&FanSpeed::High), "speed_3");
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
        tracing::info!("Response: {} - {}", response.status(), response.text().await?);

        Ok(())
    }
}
