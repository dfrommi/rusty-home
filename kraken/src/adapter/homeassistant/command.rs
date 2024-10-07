use serialize::to_message;
use support::mqtt::MqttOutMessage;
use tokio::sync::mpsc::Sender;

use crate::adapter::CommandExecutor;

use super::HaService;

pub struct HaCommandExecutor {
    mqtt_sender: Sender<MqttOutMessage>,
    command_mqtt_topic: String,
}

impl HaCommandExecutor {
    pub fn new(mqtt_sender: Sender<MqttOutMessage>, command_mqtt_topic: &str) -> Self {
        Self {
            mqtt_sender,
            command_mqtt_topic: command_mqtt_topic.to_owned(),
        }
    }
}

impl CommandExecutor<HaService> for HaCommandExecutor {
    async fn execute_command(&self, command: &HaService) -> anyhow::Result<bool> {
        let payload = to_message(command)?;

        let mqtt_msg = MqttOutMessage {
            topic: self.command_mqtt_topic.to_owned(),
            payload,
            retain: false,
        };

        self.mqtt_sender
            .send(mqtt_msg)
            .await
            .map(|_| true)
            .map_err(Into::into)
    }
}

mod serialize {
    use serde::Serialize;

    use crate::adapter::homeassistant::HaService;

    pub fn to_message(service: &HaService) -> Result<String, serde_json::Error> {
        let message = match service {
            HaService::SwitchTurnOn { id } => HaMessage::CallService {
                domain: "switch",
                service: "turn_on",
                service_data: HaServiceData::ForEntities {
                    ids: vec![id.to_owned()],
                },
            },
            HaService::SwitchTurnOff { id } => HaMessage::CallService {
                domain: "switch",
                service: "turn_off",
                service_data: HaServiceData::ForEntities {
                    ids: vec![id.to_owned()],
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
