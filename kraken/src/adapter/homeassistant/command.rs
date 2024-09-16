use api::command::Command;

use serialize::to_message;

use super::config::ha_command_entity;

pub fn to_command_payload(command: &Command) -> Option<String> {
    let ha_command = ha_command_entity(command);

    match ha_command {
        None => {
            tracing::error!("Command not supported by HA: {:?}", command);
            None
        }
        Some(cmd) => to_message(&cmd)
            .map_err(|e| {
                tracing::error!("Internal error processing JSON: {:?}", e);
                e
            })
            .ok(),
    }
}

mod serialize {
    use serde::Serialize;

    use crate::adapter::homeassistant::{HaCommandEntity, HomeAssistantService};

    pub fn to_message(ha_command_entity: &HaCommandEntity) -> Result<String, serde_json::Error> {
        let message = match ha_command_entity.service {
            HomeAssistantService::SwitchTurnOn => HaMessage::CallService {
                domain: "switch",
                service: "turn_on",
                service_data: HaServiceData::ForEntities {
                    ids: vec![ha_command_entity.id.to_string()],
                },
            },
            HomeAssistantService::SwitchTurnOff => HaMessage::CallService {
                domain: "switch",
                service: "turn_off",
                service_data: HaServiceData::ForEntities {
                    ids: vec![ha_command_entity.id.to_string()],
                },
            },
        };

        serde_json::to_string(&message)
    }

    #[derive(Serialize, Debug)]
    #[serde(tag = "event_type", content = "event_data")]
    enum HaMessage {
        #[serde(rename = "call_service")]
        CallService {
            domain: &'static str,
            service: &'static str,
            service_data: HaServiceData,
        },
    }

    #[derive(Serialize, Debug)]
    #[serde(untagged)]
    enum HaServiceData {
        ForEntities {
            #[serde(rename = "entity_id")]
            ids: Vec<String>,
        },
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
