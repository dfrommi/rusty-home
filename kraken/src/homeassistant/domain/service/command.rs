use api::command::{Command, CommandTarget};
use serde_json::json;
use support::{time::Duration, unit::DegreeCelsius};

use crate::{
    core::CommandExecutor,
    homeassistant::domain::{port::CallServicePort, HaServiceTarget},
};

pub struct HaCommandExecutor<C> {
    client: C,
    config: Vec<(CommandTarget, HaServiceTarget)>,
}

impl<C> HaCommandExecutor<C> {
    pub fn new(client: C, config: &[(CommandTarget, HaServiceTarget)]) -> Self {
        let mut data: Vec<(CommandTarget, HaServiceTarget)> = Vec::new();

        for (cmd, ha) in config {
            data.push((cmd.clone(), ha.clone()));
        }

        Self {
            client,
            config: data,
        }
    }
}

impl<C: CallServicePort> CommandExecutor for HaCommandExecutor<C> {
    #[tracing::instrument(name = "execute_command HA", ret, skip(self))]
    async fn execute_command(&self, command: &Command) -> anyhow::Result<bool> {
        let command_target: CommandTarget = command.clone().into();

        //could be more efficient, but would require eq and hash on CommandTarget
        let ha_target = self.config.iter().find_map(|(cmd, ha)| {
            if cmd == &command_target {
                Some(ha)
            } else {
                None
            }
        });

        if ha_target.is_none() {
            return Ok(false);
        }

        let ha_target = ha_target.unwrap();

        self.dispatch_service_call(command, ha_target)
            .await
            .map(|_| true)
    }
}

impl<C: CallServicePort> HaCommandExecutor<C> {
    async fn dispatch_service_call(
        &self,
        command: &Command,
        ha_target: &HaServiceTarget,
    ) -> anyhow::Result<()> {
        use api::command::*;
        use HaServiceTarget::*;

        match (ha_target, command) {
            (LightTurnOnOff(id), Command::SetPower { power_on, .. }) => {
                self.light_turn_on_off(id, *power_on).await
            }
            (
                ClimateControl(id),
                Command::SetHeating {
                    target_state: HeatingTargetState::Off,
                    ..
                },
            ) => self.climate_set_hvac_mode(id, "off").await,
            (
                ClimateControl(id),
                Command::SetHeating {
                    target_state: HeatingTargetState::Auto,
                    ..
                },
            ) => self.climate_set_hvac_mode(id, "auto").await,
            (
                ClimateControl(id),
                Command::SetHeating {
                    target_state:
                        HeatingTargetState::Heat {
                            temperature,
                            duration,
                        },
                    ..
                },
            ) => self.tado_set_climate_timer(id, temperature, duration).await,
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
            (LgWebosSmartTv(id), Command::SetEnergySaving { on, .. }) => {
                self.lg_tv_energy_saving_mode(id, *on).await
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

    async fn climate_set_hvac_mode(&self, id: &str, mode: &str) -> anyhow::Result<()> {
        self.client
            .call_service(
                "climate",
                "set_hvac_mode",
                json!({
                    "entity_id": vec![id.to_string()],
                    "hvac_mode": mode,
                }),
            )
            .await
    }

    async fn tado_set_climate_timer(
        &self,
        id: &str,
        temperature: &DegreeCelsius,
        duration: &Duration,
    ) -> anyhow::Result<()> {
        self.client
            .call_service(
                "tado",
                "set_climate_timer",
                json!({
                    "entity_id": vec![id.to_string()],
                    "temperature": temperature,
                    "time_period": to_ha_duration_format(duration),
                }),
            )
            .await
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

fn to_ha_duration_format(duration: &Duration) -> String {
    let total_seconds = duration.as_secs();
    let hh = total_seconds / 3600;
    let mm = (total_seconds % 3600) / 60;
    let ss = total_seconds % 60;

    format!("{:02}:{:02}:{:02}", hh, mm, ss)
}

fn luna_send_payload(entity_id: &str, uri: &str, payload: serde_json::Value) -> serde_json::Value {
    let luna_url = format!("luna://{}", uri);

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
