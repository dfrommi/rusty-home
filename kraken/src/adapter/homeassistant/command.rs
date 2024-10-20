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
    use std::collections::HashMap;

    use chrono::{Duration, Utc};
    use serde::Serialize;
    use serde_json::{json, Value};

    use crate::adapter::homeassistant::{HaClimateHvacMode, HaService};

    pub fn to_message(service: &HaService) -> Result<String, serde_json::Error> {
        let message = match service {
            HaService::SwitchTurnOnOff { id, power_on } => HaMessage::CallService {
                domain: "switch",
                service: if *power_on { "turn_on" } else { "turn_off" },
                service_data: HaServiceData::ForEntities {
                    ids: vec![id.to_owned()],
                    extra: HashMap::new(),
                },
            },
            HaService::LightTurnOnOff { id, power_on } => HaMessage::CallService {
                domain: "light",
                service: if *power_on { "turn_on" } else { "turn_off" },
                service_data: HaServiceData::ForEntities {
                    ids: vec![id.to_owned()],
                    extra: HashMap::new(),
                },
            },
            HaService::ClimateSetHvacMode { id, mode } => HaMessage::CallService {
                domain: "climate",
                service: "set_hvac_mode",
                service_data: HaServiceData::ForEntities {
                    ids: vec![id.to_owned()],
                    extra: HashMap::from([(
                        "mode".to_string(),
                        json!(match mode {
                            HaClimateHvacMode::Off => "off",
                            HaClimateHvacMode::Auto => "auto",
                        }),
                    )]),
                },
            },
            HaService::TadoSetClimateTimer {
                id,
                temperature,
                until,
            } => HaMessage::CallService {
                domain: "tado",
                service: "set_climate_timer",
                service_data: HaServiceData::ForEntities {
                    ids: vec![id.to_owned()],
                    extra: HashMap::from([
                        ("temperature".to_string(), json!(temperature)),
                        (
                            "time_period".to_string(),
                            json!(to_ha_duration_format(
                                until.signed_duration_since(Utc::now())
                            )),
                        ),
                    ]),
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

            #[serde(flatten)]
            extra: HashMap<String, Value>,
        },
    }

    fn to_ha_duration_format(duration: Duration) -> String {
        let total_seconds = duration.num_seconds();
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
