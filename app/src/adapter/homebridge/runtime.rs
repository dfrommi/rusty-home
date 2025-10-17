use std::collections::HashMap;

use infrastructure::{MqttInMessage, MqttOutMessage};
use serde::{Deserialize, Serialize};
use tokio::{
    sync::broadcast::{Receiver, error::RecvError},
    task::JoinHandle,
};

use crate::{
    adapter::homebridge::{
        HomekitEvent, HomekitService, HomekitTarget, HomekitTargetConfig, accessory::HomekitRegistry,
        hap::HomekitCharacteristic,
    },
    core::{HomeApi, timeseries::DataPoint},
    home::{state::HomeStateValue, trigger::UserTrigger},
};

pub struct HomebridgeRunner {
    registry: HomekitRegistry,
    state_change_rx: Receiver<DataPoint<HomeStateValue>>,
    mqtt_sender: tokio::sync::mpsc::Sender<MqttOutMessage>,
    mqtt_receiver: tokio::sync::mpsc::Receiver<MqttInMessage>,
    mqtt_base_topic: String,
    api: HomeApi,
    trigger_debounce: HashMap<HomekitTarget, JoinHandle<()>>,
}

impl HomebridgeRunner {
    pub fn new(
        registry: HomekitRegistry,
        state_change_rx: Receiver<DataPoint<HomeStateValue>>,
        mqtt_sender: tokio::sync::mpsc::Sender<MqttOutMessage>,
        mqtt_receiver: tokio::sync::mpsc::Receiver<MqttInMessage>,
        mqtt_base_topic: String,
        api: HomeApi,
    ) -> Self {
        Self {
            registry,
            state_change_rx,
            mqtt_sender,
            mqtt_receiver,
            mqtt_base_topic,
            api,
            trigger_debounce: HashMap::new(),
        }
    }

    pub async fn run(mut self) {
        self.register_accessory().await;

        loop {
            tokio::select! {
                Some(mqtt_msg) = self.mqtt_receiver.recv() => {
                    self.handle_mqtt_message(mqtt_msg).await;
                }

                state_change = self.state_change_rx.recv() => {
                    match state_change {
                        Ok(state) => {
                            self.handle_state_change(state).await;
                        }
                        Err(RecvError::Closed) => {
                            tracing::error!("State change receiver channel closed");
                        }
                        Err(RecvError::Lagged(count)) => {
                            tracing::warn!("State change receiver lagged by {} messages", count);
                        }
                    }
                }
            }
        }
    }

    async fn handle_state_change(&mut self, state: DataPoint<HomeStateValue>) {
        //example
        // {"name": "flex_lamp", "service_name": "light", "characteristic": "On", "value": true}
        #[derive(Debug, Serialize)]
        struct OutgoingMessage {
            name: String,
            #[serde(rename = "service_name")]
            service: HomekitService,
            characteristic: HomekitCharacteristic,
            value: serde_json::Value,
        }

        let exports = self
            .registry
            .export_state(&state.value)
            .into_iter()
            .map(|export| OutgoingMessage {
                name: export.target.name,
                service: export.target.service,
                characteristic: export.target.characteristic,
                value: export.value,
            })
            .collect::<Vec<OutgoingMessage>>();

        for export in exports {
            let topic = format!("{}/to/set", self.mqtt_base_topic);
            let payload = match serde_json::to_string(&export) {
                Ok(p) => p,
                Err(e) => {
                    tracing::error!("Error serializing Homebridge outgoing message: {:?} -- {:?}", export, e);
                    continue;
                }
            };
            let msg = MqttOutMessage::transient(topic.to_string(), payload);
            if let Err(e) = self.mqtt_sender.send(msg).await {
                tracing::error!("Error sending MQTT message to Homebridge: {} -- {:?}", topic, e);
            }
        }
    }

    async fn handle_mqtt_message(&mut self, msg: MqttInMessage) {
        //example
        // {"name": "flex_lamp", "service_name": "light", "characteristic": "On", "value": true}
        #[derive(Deserialize, Debug)]
        struct IncomingMessage {
            name: String,
            #[serde(rename = "service_name")]
            service: HomekitService,
            characteristic: HomekitCharacteristic,
            value: serde_json::Value,
        }

        let incoming: IncomingMessage = match serde_json::from_str(&msg.payload) {
            Ok(msg) => msg,
            Err(e) => {
                tracing::error!("Error parsing incoming Homebridge message: {:?} -- {:?}", msg.payload, e);
                return;
            }
        };

        let state = HomekitEvent {
            target: HomekitTarget::new(incoming.name, incoming.service, incoming.characteristic),
            value: incoming.value,
        };

        tracing::debug!("Processing Homebridge MQTT event: {:?}", state);

        if let Some(command) = self.registry.process_trigger(&state) {
            if let Some(handle) = self.trigger_debounce.get(&state.target) {
                handle.abort();
            }

            tracing::info!("Debouncing Homebridge command for target: {:?}", state.target);

            let api = self.api.clone();
            let handle = tokio::spawn(async move {
                tokio::time::sleep(std::time::Duration::from_millis(2000)).await;

                tracing::info!("Received Homebridge command: {:?}", command);
                if let Err(e) = api.add_user_trigger(UserTrigger::Homekit(command.clone())).await {
                    tracing::error!("Error processing Homekit command {:?}: {:?}", command, e);
                }
            });

            self.trigger_debounce.insert(state.target, handle);
        }
    }

    async fn register_accessory(&mut self) {
        let bootstrap_data = self.get_bootstrap_data();

        let mut already_registered: Vec<String> = vec![];

        for ((name, service), characteristics) in bootstrap_data {
            //make sure accessory is created before service is added
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;

            let topic = if already_registered.contains(&name) {
                format!("{}/to/add/service", self.mqtt_base_topic)
            } else {
                already_registered.push(name.clone());
                format!("{}/to/add", self.mqtt_base_topic)
            };

            let payload = Self::service_registration_payload(name.clone(), service.clone(), &characteristics);
            let msg = MqttOutMessage::transient(topic.to_string(), payload.to_string());

            if let Err(e) = self.mqtt_sender.send(msg).await {
                tracing::error!("Error sending MQTT message to Homebridge: {} -- {:?}", topic, e);
            }
        }
    }

    fn get_bootstrap_data(&mut self) -> HashMap<(String, HomekitService), Vec<HomekitTargetConfig>> {
        self.registry
            .get_device_config()
            .into_iter()
            .map(|entry| {
                let key = (entry.target.name.clone(), entry.target.service.clone());
                (key, entry)
            })
            .fold(HashMap::new(), |mut acc, (key, entry)| {
                acc.entry(key).or_default().push(entry);
                acc
            })
    }

    fn service_registration_payload(
        name: String,
        service: HomekitService,
        characteristics: &[HomekitTargetConfig],
    ) -> serde_json::Value {
        #[derive(Serialize)]
        struct Payload {
            name: String,
            service_name: HomekitService,
            service: HomekitService,
            #[serde(flatten)]
            config: HashMap<HomekitCharacteristic, serde_json::Value>,
        }

        let mut config = HashMap::<HomekitCharacteristic, serde_json::Value>::new();
        for characteristic in characteristics.iter() {
            let value = characteristic
                .config
                .clone()
                .unwrap_or_else(|| serde_json::Value::String("default".to_string()));
            config.insert(characteristic.target.characteristic.clone(), value);
        }

        let payload = Payload {
            name,
            service_name: service.clone(),
            service,
            config,
        };

        serde_json::to_value(payload).expect("Error serializing Homebridge service registration payload")
    }
}

#[cfg(test)]
mod tests {
    use assert_json_diff::assert_json_eq;

    use super::*;

    #[test]
    fn test_service_registration_payload() {
        let name = "Test Sensor".to_string();
        let service = HomekitService::TemperatureSensor;
        let characteristics = vec![
            HomekitTarget::new(name.clone(), service.clone(), HomekitCharacteristic::CurrentTemperature).into_config(),
        ];

        let payload = HomebridgeRunner::service_registration_payload(name.clone(), service.clone(), &characteristics);

        assert_json_eq!(
            payload,
            serde_json::json!({
                "name": name,
                "service_name": "TemperatureSensor",
                "service": "TemperatureSensor",
                "CurrentTemperature": "default"
            })
        );
    }
}
