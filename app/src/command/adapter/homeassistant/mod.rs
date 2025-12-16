mod config;

use infrastructure::HttpClientConfig;
use reqwest_middleware::ClientWithMiddleware;

use crate::command::adapter::CommandExecutor;
use crate::command::{Command, CommandTarget, Fan};
use crate::core::unit::{FanAirflow, FanSpeed};
use serde_json::json;

#[derive(Debug, Clone)]
enum HaServiceTarget {
    LightTurnOnOff(&'static str),
    PushNotification(&'static str),
    LgWebosSmartTv(&'static str),
    WindcalmFanSpeed(&'static str),
}

pub struct HomeAssistantCommandExecutor {
    client: HaHttpClient,
    config: Vec<(CommandTarget, HaServiceTarget)>,
}

impl HomeAssistantCommandExecutor {
    pub fn new(url: &str, token: &str) -> Self {
        let http_client = HaHttpClient::new(url, token).expect("Error initializing Home Assistant REST client");

        let mut data: Vec<(CommandTarget, HaServiceTarget)> = Vec::new();

        for (cmd, ha) in config::default_ha_command_config() {
            data.push((cmd.clone(), ha.clone()));
        }

        Self {
            client: http_client,
            config: data,
        }
    }
}

impl CommandExecutor for HomeAssistantCommandExecutor {
    #[tracing::instrument(name = "execute_command HA", ret, skip(self))]
    async fn execute_command(&self, command: &Command) -> anyhow::Result<bool> {
        let command_target: CommandTarget = command.clone().into();

        //could be more efficient, but would require eq and hash on CommandTarget
        let ha_target = self
            .config
            .iter()
            .find_map(|(cmd, ha)| if cmd == &command_target { Some(ha) } else { None });

        if ha_target.is_none() {
            return Ok(false);
        }

        let ha_target = ha_target.unwrap();

        self.dispatch_service_call(command, ha_target).await.map(|_| true)
    }
}

impl HomeAssistantCommandExecutor {
    async fn dispatch_service_call(&self, command: &Command, ha_target: &HaServiceTarget) -> anyhow::Result<()> {
        use crate::command::*;
        use HaServiceTarget::*;

        match (ha_target, command) {
            (LightTurnOnOff(id), Command::SetPower { power_on, .. }) => self.light_turn_on_off(id, *power_on).await,
            (
                PushNotification(mobile_id),
                Command::PushNotify {
                    notification: Notification::WindowOpened,
                    action: NotificationAction::Notify,
                    ..
                },
            ) => self.notify_window_opened(mobile_id).await,
            (
                PushNotification(mobile_id),
                Command::PushNotify {
                    notification: Notification::WindowOpened,
                    action: NotificationAction::Dismiss,
                    ..
                },
            ) => self.dismiss_window_opened_notification(mobile_id).await,
            (LgWebosSmartTv(id), Command::SetEnergySaving { on, .. }) => self.lg_tv_energy_saving_mode(id, *on).await,
            (WindcalmFanSpeed(id), Command::ControlFan { device, speed }) => {
                self.windcalm_fan_speed(id, device, speed).await
            }
            conf => Err(anyhow::anyhow!("Invalid configuration: {:?}", conf,)),
        }
    }

    async fn light_turn_on_off(&self, id: &str, power_on: bool) -> anyhow::Result<()> {
        let service = if power_on { "turn_on" } else { "turn_off" };
        self.client
            .call_service(
                "light",
                service,
                json!({
                    "entity_id": vec![id.to_string()],
                }),
            )
            .await
    }

    async fn windcalm_fan_speed(&self, id: &str, fan: &Fan, airflow: &FanAirflow) -> anyhow::Result<()> {
        fn to_percent(fan_speed: &FanSpeed) -> usize {
            match fan_speed {
                FanSpeed::Silent => 1,
                FanSpeed::Low => 21,
                FanSpeed::Medium => 41,
                FanSpeed::High => 61,
                FanSpeed::Turbo => 81,
            }
        }

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
                    .await?
            }
            FanAirflow::Forward(fan_speed) | FanAirflow::Reverse(fan_speed) => {
                let direction = match airflow {
                    FanAirflow::Forward(_) => "forward",
                    FanAirflow::Reverse(_) => "reverse",
                    _ => unreachable!(),
                };
                self.client
                    .call_service(
                        "fan",
                        "set_direction",
                        json!({
                            "entity_id": vec![id.to_string()],
                            "direction": direction
                        }),
                    )
                    .await?;
                self.client
                    .call_service(
                        "fan",
                        "turn_on",
                        json!({
                            "entity_id": vec![id.to_string()],
                            "percentage": to_percent(fan_speed)
                        }),
                    )
                    .await?
            }
        };

        Ok(())
    }

    async fn notify_window_opened(&self, mobile_id: &str) -> anyhow::Result<()> {
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
            .await
    }

    async fn dismiss_window_opened_notification(&self, mobile_id: &str) -> anyhow::Result<()> {
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
            .await
    }

    async fn lg_tv_energy_saving_mode(&self, id: &str, energy_saving: bool) -> anyhow::Result<()> {
        let luna_result = self
            .client
            .call_service(
                "webostv",
                "command",
                luna_send_payload(
                    id,
                    "com.webos.settingsservice/setSystemSettings",
                    json!({
                        "category": "picture",
                        "settings": {
                            "energySaving": if energy_saving { "auto" } else { "off" },
                            "energySavingModified": "true"
                        }
                    }),
                ),
            )
            .await;

        if luna_result.is_ok() {
            tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
            self.client
                .call_service(
                    "webostv",
                    "button",
                    json!({
                        "entity_id": vec![id.to_string()],
                        "button": "ENTER"
                    }),
                )
                .await?;

            if !energy_saving {
                tokio::time::sleep(tokio::time::Duration::from_millis(250)).await;
                self.client
                    .call_service(
                        "webostv",
                        "button",
                        json!({
                            "entity_id": vec![id.to_string()],
                            "button": "ENTER"
                        }),
                    )
                    .await?;
            }
        }

        Ok(())
    }
}

fn luna_send_payload(entity_id: &str, uri: &str, payload: serde_json::Value) -> serde_json::Value {
    let luna_url = format!("luna://{uri}");

    json!({
        "entity_id": vec![entity_id.to_string()],
        "command": "system.notifications/createAlert",
        "payload": {
            "message": " ",
            "buttons": [{
                    "label": "",
                    "onClick": luna_url,
                    "params": payload,
            }],
            "onclose": {"uri": luna_url, "params": payload},
            "onfail": {"uri": luna_url, "params": payload},
        }
    })
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
