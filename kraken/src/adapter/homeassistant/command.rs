use api::command::{Command, CommandExecution, CommandTarget};
use serialize::{to_message, HaMessage};

use crate::port::CommandExecutor;

use super::{HaRestClient, HaServiceTarget};

pub struct HaCommandExecutor {
    client: HaRestClient,
    config: Vec<(CommandTarget, HaServiceTarget)>,
}

impl HaCommandExecutor {
    pub fn new(client: HaRestClient, config: &[(CommandTarget, HaServiceTarget)]) -> Self {
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

impl CommandExecutor for HaCommandExecutor {
    async fn execute_command(&self, command: &CommandExecution<Command>) -> anyhow::Result<bool> {
        let command_target: CommandTarget = command.command.clone().into();

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

        let payload = to_message(&command.command, ha_target)?;

        match payload {
            HaMessage::CallService {
                domain,
                service,
                service_data,
            } => {
                self.client
                    .call_service(domain, service, serde_json::to_value(service_data)?)
                    .await?;
                Ok(true)
            }
        }
    }
}

mod serialize {
    use std::collections::HashMap;

    use api::command::{Command, HeatingTargetState, SetHeating, SetPower};
    use serde::Serialize;
    use serde_json::{json, Value};
    use support::time::Duration;

    use crate::adapter::homeassistant::HaServiceTarget;

    //TODO simplify HaMessage struct, maybe use new
    pub fn to_message(command: &Command, ha_target: &HaServiceTarget) -> anyhow::Result<HaMessage> {
        use HaServiceTarget::*;

        let message = match (ha_target, command) {
            (SwitchTurnOnOff(id), Command::SetPower(SetPower { power_on, .. })) => {
                HaMessage::CallService {
                    domain: "switch",
                    service: if *power_on { "turn_on" } else { "turn_off" },
                    service_data: HaServiceData::ForEntities {
                        ids: vec![id.to_owned()],
                        extra: HashMap::new(),
                    },
                }
            }
            (LightTurnOnOff(id), Command::SetPower(SetPower { power_on, .. })) => {
                HaMessage::CallService {
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
            ) => HaMessage::CallService {
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
            ) => HaMessage::CallService {
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
            ) => HaMessage::CallService {
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
    #[serde(tag = "event_type", content = "event_data")]
    pub enum HaMessage {
        #[serde(rename = "call_service")]
        CallService {
            domain: &'static str,
            service: &'static str,
            service_data: HaServiceData,
        },
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

    #[cfg(test)]
    mod tests {
        use assert_json_diff::assert_json_eq;
        use serde_json::json;

        use super::*;

        #[test]
        fn serialze_command() {
            //GIVEN
            let command = HaMessage::CallService {
                domain: "testdomain",
                service: "testservice",
                service_data: HaServiceData::ForEntities {
                    ids: vec!["my_switch".to_string()],
                    extra: HashMap::new(),
                },
            };

            let expected_json = json!({
                "event_type": "call_service",
                "event_data": {
                    "domain": "testdomain",
                    "service": "testservice",
                    "service_data": {
                        "entity_id": ["my_switch"]
                    }
                }
            });

            //WHEN
            let serialized = serde_json::to_value(command).unwrap();

            //THEN
            assert_json_eq!(&serialized, &expected_json)
        }
    }
}
