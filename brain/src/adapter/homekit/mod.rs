mod command;
mod state;

use crate::home::state::FanSpeed;
use serde::Deserialize;
use support::unit::Percent;

use crate::Infrastructure;

#[derive(Debug, Deserialize, Clone)]
#[allow(unused)]
pub struct Homekit {
    pub base_topic_status: String,
    pub base_topic_set: String,
}

impl Homekit {
    pub fn export_state(
        &self,
        infrastructure: &Infrastructure,
    ) -> impl Future<Output = ()> + use<> {
        let mqtt_api = infrastructure.database.clone();
        let mqtt_sender = infrastructure.mqtt_client.new_publisher();
        let state_topic = self.base_topic_status.clone();
        let mqtt_trigger = infrastructure.event_listener.new_state_changed_listener();

        async move { state::export_state(&mqtt_api, state_topic, mqtt_sender, mqtt_trigger).await }
    }

    pub async fn process_commands(
        &self,
        infrastructure: &mut Infrastructure,
    ) -> impl Future<Output = ()> + use<> {
        let mqtt_command_receiver = infrastructure
            .mqtt_client
            .subscribe(format!("{}/#", &self.base_topic_set))
            .await
            .expect("Error subscribing to MQTT topic");
        let api = infrastructure.database.clone();
        let target_topic = self.base_topic_set.clone();

        async move { command::process_commands(target_topic, mqtt_command_receiver, api).await }
    }
}

#[derive(Debug, Clone)]
struct MqttStateValue(String);

impl From<bool> for MqttStateValue {
    fn from(val: bool) -> Self {
        MqttStateValue(if val {
            "1".to_string()
        } else {
            "0".to_string()
        })
    }
}

impl TryInto<bool> for MqttStateValue {
    type Error = anyhow::Error;

    fn try_into(self) -> Result<bool, Self::Error> {
        match self.0.as_str() {
            "0" => Ok(false),
            "1" => Ok(true),
            _ => anyhow::bail!("Error converting {} to bool", self.0),
        }
    }
}

impl From<Percent> for MqttStateValue {
    fn from(val: Percent) -> Self {
        MqttStateValue(val.0.to_string())
    }
}

impl TryInto<Percent> for MqttStateValue {
    type Error = anyhow::Error;

    fn try_into(self) -> Result<Percent, Self::Error> {
        match self.0.parse() {
            Ok(v) => Ok(Percent(v)),
            Err(_) => anyhow::bail!("Error converting {} to Percent", self.0),
        }
    }
}

impl From<FanSpeed> for MqttStateValue {
    fn from(val: FanSpeed) -> Self {
        MqttStateValue(
            match val {
                FanSpeed::Silent => "20",
                FanSpeed::Low => "40",
                FanSpeed::Medium => "60.0",
                FanSpeed::High => "80.0",
                FanSpeed::Turbo => "100",
            }
            .to_string(),
        )
    }
}

impl TryInto<FanSpeed> for MqttStateValue {
    type Error = anyhow::Error;

    fn try_into(self) -> Result<FanSpeed, Self::Error> {
        let percent: Percent = self.try_into()?;

        if percent.0 == 0.0 {
            anyhow::bail!("Fan speed is 0.0, not a fan speed, should be OFF");
        }

        Ok(if percent.0 <= 20.0 {
            FanSpeed::Silent
        } else if percent.0 <= 40.0 {
            FanSpeed::Low
        } else if percent.0 <= 60.0 {
            FanSpeed::Medium
        } else if percent.0 <= 80.0 {
            FanSpeed::High
        } else {
            FanSpeed::Turbo
        })
    }
}
