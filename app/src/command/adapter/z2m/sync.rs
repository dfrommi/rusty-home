use crate::core::domain::Radiator;
use crate::core::math::round_to_one_decimal;
use crate::core::timeseries::DataPoint;
use crate::home_state::{HomeStateEvent, HomeStateValue};
use infrastructure::{EventListener, MqttSender};
use tracing::Level;

pub struct Z2mSensorSyncRunner {
    sonoff_devices: Vec<SonoffThermostatExtTempSync>,
    home_state_events: EventListener<HomeStateEvent>,
}

impl Z2mSensorSyncRunner {
    pub fn new(mqtt_sender: MqttSender, home_state_events: EventListener<HomeStateEvent>) -> Self {
        Self {
            sonoff_devices: Radiator::variants()
                .iter()
                .map(|r| SonoffThermostatExtTempSync::new(*r, mqtt_sender.clone()))
                .collect(),
            home_state_events,
        }
    }

    pub async fn run(mut self) {
        loop {
            if let Some(event) = self.home_state_events.recv().await {
                for sonoff_device in &self.sonoff_devices {
                    sonoff_device.handle_home_state_event(&event).await;
                }
            }
        }
    }
}

struct SonoffThermostatExtTempSync {
    radiator: Radiator,
    mqtt_sender: MqttSender,
    device_id: String,
    set_topic: String,
}

impl SonoffThermostatExtTempSync {
    fn new(radiator: Radiator, mqtt_sender: MqttSender) -> Self {
        let device_id = match radiator {
            Radiator::Bedroom => "bedroom/radiator_thermostat_sonoff",
            Radiator::LivingRoomBig => "living_room/radiator_thermostat_big_sonoff",
            Radiator::LivingRoomSmall => "living_room/radiator_thermostat_small_sonoff",
            Radiator::RoomOfRequirements => "room_of_requirements/radiator_thermostat_sonoff",
            Radiator::Kitchen => "kitchen/radiator_thermostat_sonoff",
            Radiator::Bathroom => "bathroom/radiator_thermostat_sonoff",
        };

        Self {
            radiator,
            mqtt_sender,
            device_id: device_id.to_string(),
            set_topic: format!("{}/set", device_id),
        }
    }

    #[tracing::instrument(level = Level::TRACE, name = "sonoff_set_temperature", skip(self, event), fields(device_id = %self.device_id))]
    async fn handle_home_state_event(&self, event: &HomeStateEvent) {
        match event {
            HomeStateEvent::Changed(DataPoint {
                value: HomeStateValue::Temperature(id, temp),
                ..
            }) if *id == self.radiator.room_temperature() => {
                let temp = round_to_one_decimal(temp.0);

                tracing::debug!(device_id = %self.device_id, "External temperature update for {}: temperature {}", self.device_id, temp);
                let payload = serde_json::json!({
                    "external_temperature_input": temp,
                    "temperature_sensor_select": "external"
                });

                self.mqtt_sender.send_transient(&self.set_topic, payload.to_string()).await.unwrap_or_else(|e| {
                    tracing::error!(device_id = %self.device_id, "Failed to publish temperature update for Sonoff thermostat {}: {}", self.device_id, e);
                });
            }
            _ => { /* Ignore other events */ }
        }
    }
}
