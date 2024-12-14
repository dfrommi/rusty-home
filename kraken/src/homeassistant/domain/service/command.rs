use api::command::{Command, CommandTarget};

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
            tracing::debug!(
                "No HA service configured for command target {:?}",
                command_target
            );
            return Ok(false);
        }

        let ha_target = ha_target.unwrap();

        let request = serialize::to_service_call_request(command, ha_target)?;

        self.client
            .call_service(request.domain, request.service, request.payload)
            .await
            .map(|_| true)
    }
}

mod serialize {
    use api::command::{
        Command, HeatingTargetState, Notification, NotificationAction, PushNotify, SetHeating,
        SetPower,
    };
    use serde_json::json;
    use support::time::Duration;

    use crate::homeassistant::domain::HaServiceTarget;

    pub struct CallServiceRequest {
        pub domain: &'static str,
        pub service: &'static str,
        pub payload: serde_json::Value,
    }

    //TODO simplify HaMessage struct, maybe use new
    pub fn to_service_call_request(
        command: &Command,
        ha_target: &HaServiceTarget,
    ) -> anyhow::Result<CallServiceRequest> {
        use HaServiceTarget::*;

        let message = match (ha_target, command) {
            (SwitchTurnOnOff(id), Command::SetPower(SetPower { power_on, .. })) => {
                CallServiceRequest {
                    domain: "switch",
                    service: if *power_on { "turn_on" } else { "turn_off" },
                    payload: json!({
                        "entity_id": vec![id.to_string()],
                    }),
                }
            }
            (LightTurnOnOff(id), Command::SetPower(SetPower { power_on, .. })) => {
                CallServiceRequest {
                    domain: "light",
                    service: if *power_on { "turn_on" } else { "turn_off" },
                    payload: json!({
                        "entity_id": vec![id.to_string()],
                    }),
                }
            }
            (
                ClimateControl(id),
                Command::SetHeating(SetHeating {
                    target_state: HeatingTargetState::Off,
                    ..
                }),
            ) => CallServiceRequest {
                domain: "climate",
                service: "set_hvac_mode",
                payload: json!({
                    "entity_id": vec![id.to_string()],
                    "hvac_mode": "off",
                }),
            },
            (
                ClimateControl(id),
                Command::SetHeating(SetHeating {
                    target_state: HeatingTargetState::Auto,
                    ..
                }),
            ) => CallServiceRequest {
                domain: "climate",
                service: "set_hvac_mode",
                payload: json!({
                    "entity_id": vec![id.to_string()],
                    "hvac_mode": "auto",
                }),
            },
            (
                ClimateControl(id),
                Command::SetHeating(SetHeating {
                    target_state: HeatingTargetState::Heat { temperature, until },
                    ..
                }),
            ) => CallServiceRequest {
                domain: "tado",
                service: "set_climate_timer",
                payload: json!({
                    "entity_id": vec![id.to_string()],
                    "temperature": temperature,
                    "time_period": to_ha_duration_format(Duration::until(until)),
                }),
            },
            (
                PushNotification(mobile_id),
                Command::PushNotify(PushNotify {
                    notification: Notification::WindowOpened,
                    action: NotificationAction::Notify,
                    ..
                }),
            ) => CallServiceRequest {
                domain: "notify",
                service: mobile_id,
                payload: json!({
                    "title": "Fenster offen",
                    "message": "Mindestens ein Fenster ist offen",
                    "data": {
                        "tag": "window_opened"
                    }
                }),
            },
            (
                PushNotification(mobile_id),
                Command::PushNotify(PushNotify {
                    notification: Notification::WindowOpened,
                    action: NotificationAction::Dismiss,
                    ..
                }),
            ) => CallServiceRequest {
                domain: "notify",
                service: mobile_id,
                payload: json!({
                    "message": "clear_notification",
                    "data": {
                        "tag": "window_opened"
                    }
                }),
            },
            conf => return Err(anyhow::anyhow!("Invalid configuration: {:?}", conf,)),
        };

        Ok(message)
    }

    fn to_ha_duration_format(duration: Duration) -> String {
        let total_seconds = duration.as_secs();
        let hh = total_seconds / 3600;
        let mm = (total_seconds % 3600) / 60;
        let ss = total_seconds % 60;

        format!("{:02}:{:02}:{:02}", hh, mm, ss)
    }
}
