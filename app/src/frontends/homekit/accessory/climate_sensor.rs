use super::Accessory;
use crate::{
    frontends::homekit::{HomekitCharacteristic, HomekitEvent, HomekitService, HomekitTarget, HomekitTargetConfig},
    home_state::{HomeStateValue, RelativeHumidity, Temperature},
};

pub struct ClimateSensor {
    name: &'static str,
    temperature: Temperature,
    humidity: RelativeHumidity,
}

impl ClimateSensor {
    pub fn new(name: &'static str, temperature: Temperature, humidity: RelativeHumidity) -> Self {
        Self {
            name,
            temperature,
            humidity,
        }
    }
}

impl Accessory for ClimateSensor {
    fn get_all_targets(&self) -> Vec<HomekitTargetConfig> {
        vec![
            HomekitTarget::new(
                self.name.to_string(),
                HomekitService::HumiditySensor,
                HomekitCharacteristic::CurrentRelativeHumidity,
            )
            .into_config(),
            HomekitTarget::new(
                self.name.to_string(),
                HomekitService::TemperatureSensor,
                HomekitCharacteristic::CurrentTemperature,
            )
            .into_config(),
        ]
    }

    fn export_state(&mut self, state: &HomeStateValue) -> Vec<HomekitEvent> {
        match state {
            HomeStateValue::Temperature(temperature, celsius) if *temperature == self.temperature => {
                vec![HomekitEvent {
                    target: HomekitTarget::new(
                        self.name.to_string(),
                        HomekitService::TemperatureSensor,
                        HomekitCharacteristic::CurrentTemperature,
                    ),
                    value: serde_json::json!(celsius.0),
                }]
            }
            HomeStateValue::RelativeHumidity(humidity, percent) if *humidity == self.humidity => {
                vec![HomekitEvent {
                    target: HomekitTarget::new(
                        self.name.to_string(),
                        HomekitService::HumiditySensor,
                        HomekitCharacteristic::CurrentRelativeHumidity,
                    ),
                    value: serde_json::json!(percent.0),
                }]
            }
            _ => Vec::new(),
        }
    }
}
