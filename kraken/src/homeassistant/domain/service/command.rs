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

        let payload = serialize::to_message(command, ha_target)?;

        self.client
            .call_service(
                payload.domain,
                payload.service,
                serde_json::to_value(payload.service_data)?,
            )
            .await
            .map(|_| true)
    }
}

mod serialize {
    use std::collections::HashMap;

    use api::command::{Command, HeatingTargetState, SetHeating, SetPower};
    use serde::Serialize;
    use serde_json::{json, Value};
    use support::time::Duration;

    use crate::homeassistant::domain::HaServiceTarget;

    pub struct CallServiceRequest {
        pub domain: &'static str,
        pub service: &'static str,
        pub service_data: HaServiceData,
    }

    //TODO simplify HaMessage struct, maybe use new
    pub fn to_message(
        command: &Command,
        ha_target: &HaServiceTarget,
    ) -> anyhow::Result<CallServiceRequest> {
        use HaServiceTarget::*;

        let message = match (ha_target, command) {
            (SwitchTurnOnOff(id), Command::SetPower(SetPower { power_on, .. })) => {
                CallServiceRequest {
                    domain: "switch",
                    service: if *power_on { "turn_on" } else { "turn_off" },
                    service_data: HaServiceData::ForEntities {
                        ids: vec![id.to_owned()],
                        extra: HashMap::new(),
                    },
                }
            }
            (LightTurnOnOff(id), Command::SetPower(SetPower { power_on, .. })) => {
                CallServiceRequest {
                    domain: "light",
                    service: if *power_on { "turn_on" } else { "turn_off" },
                    service_data: HaServiceData::ForEntities {
                        ids: vec![id.to_owned()],
                        extra: HashMap::new(),
                    },
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
                service_data: HaServiceData::ForEntities {
                    ids: vec![id.to_owned()],
                    extra: HashMap::from([("hvac_mode".to_string(), json!("off"))]),
                },
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
                service_data: HaServiceData::ForEntities {
                    ids: vec![id.to_owned()],
                    extra: HashMap::from([("hvac_mode".to_string(), json!("auto"))]),
                },
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
                service_data: HaServiceData::ForEntities {
                    ids: vec![id.to_owned()],
                    extra: HashMap::from([
                        ("temperature".to_string(), json!(temperature)),
                        (
                            "time_period".to_string(),
                            json!(to_ha_duration_format(Duration::until(until))),
                        ),
                    ]),
                },
            },
            conf => return Err(anyhow::anyhow!("Invalid configuration: {:?}", conf,)),
        };

        Ok(message)
    }

    #[derive(Serialize, Debug)]
    #[serde(untagged)]
    pub enum HaServiceData {
        ForEntities {
            #[serde(rename = "entity_id")]
            ids: Vec<String>,

            #[serde(flatten)]
            extra: HashMap<String, Value>,
        },
    }

    fn to_ha_duration_format(duration: Duration) -> String {
        let total_seconds = duration.as_secs();
        let hh = total_seconds / 3600;
        let mm = (total_seconds % 3600) / 60;
        let ss = total_seconds % 60;

        format!("{:02}:{:02}:{:02}", hh, mm, ss)
    }
}
