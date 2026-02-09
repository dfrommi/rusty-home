mod config;
pub mod sender;

use super::metrics::*;
use crate::{
    command::{Command, CommandTarget, HeatingTargetState, adapter::CommandExecutor},
    core::unit::Percent,
};
use sender::Z2mSender;
use serde_json::json;

#[derive(Debug, Clone)]
pub enum Z2mCommandTarget {
    SonoffThermostat(&'static str),
    PowerPlug(&'static str),
}

pub struct Z2mCommandExecutor {
    config: Vec<(CommandTarget, Z2mCommandTarget)>,
    sender: Z2mSender,
}

impl Z2mCommandExecutor {
    pub fn new(mqtt_sender: Z2mSender) -> Self {
        let config = config::default_z2m_command_config();
        Self {
            config,
            sender: mqtt_sender,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Z2mTopic {
    device_id: String,
}

impl Z2mTopic {
    pub fn new(device_id: &str) -> Self {
        Self {
            device_id: device_id.trim_matches('/').to_string(),
        }
    }

    pub fn from_topic(topic: &str) -> Option<Self> {
        let topic = topic.trim_matches('/');
        let device_id = topic
            .strip_suffix("/set")
            .or_else(|| topic.strip_suffix("/get"))
            .unwrap_or(topic)
            .trim_matches('/');

        if device_id.is_empty() {
            return None;
        }

        Some(Self::new(device_id))
    }

    pub fn is_command(topic: &str) -> bool {
        topic.trim_matches('/').ends_with("/set")
    }

    pub fn is_state_update(topic: &str) -> bool {
        !Self::is_command(topic)
    }

    pub fn device_id(&self) -> &str {
        &self.device_id
    }

    pub fn command_topic(&self) -> String {
        format!("{}/set", self.device_id)
    }

    pub fn active_get_topic(&self) -> String {
        format!("{}/get", self.device_id)
    }
}

impl std::fmt::Display for Z2mTopic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.device_id())
    }
}

impl CommandExecutor for Z2mCommandExecutor {
    #[tracing::instrument(name = "execute_command Z2M", ret, skip(self))]
    async fn execute_command(&self, command: &Command) -> anyhow::Result<bool> {
        let cmd_target: CommandTarget = command.into();
        let z2m_target = self
            .config
            .iter()
            .find_map(|(cmd, z2m)| if cmd == &cmd_target { Some(z2m) } else { None });

        let Some(z2m_target) = z2m_target else {
            return Ok(false);
        };

        match (command, z2m_target) {
            (
                Command::SetThermostatValveOpeningPosition { value, .. },
                Z2mCommandTarget::SonoffThermostat(device_id),
            ) => self.set_valve_opening_position_sonoff(device_id, *value).await,
            (Command::SetPower { power_on, .. }, Z2mCommandTarget::PowerPlug(device_id)) => {
                self.set_power_state(device_id, *power_on).await
            }
            (Command::SetHeating { target_state, .. }, Z2mCommandTarget::SonoffThermostat(device_id)) => {
                self.set_sonoff_heating(device_id, target_state.clone()).await
            }
            (_, z2m_target) => {
                anyhow::bail!("Mismatch between command and Z2M target {:?}", z2m_target)
            }
        }
    }
}

impl Z2mCommandExecutor {
    pub async fn set_sonoff_heating(&self, device_id: &str, state: HeatingTargetState) -> anyhow::Result<bool> {
        match state {
            HeatingTargetState::Off => {
                self.send_message(
                    device_id,
                    vec![json!({
                        "system_mode": "off",
                        "occupied_heating_setpoint": 7,
                        "valve_opening_degree": 0,
                        "valve_closing_degree": 100,
                        "temperature_accuracy": -1,
                    })],
                    false,
                )
                .await?;

                Ok(true)
            }
            HeatingTargetState::Heat {
                target_temperature,
                demand_limit,
            } => {
                let temperature_accuracy = round_to_one_decimal_place(
                    (target_temperature.to().0 - target_temperature.from().0).clamp(0.2, 1.0),
                );

                self.send_message(
                    device_id,
                    vec![json!({
                        "system_mode": "heat",
                        "occupied_heating_setpoint": json_no_fraction_if_zero(target_temperature.to().0),
                        "valve_opening_degree": demand_limit.to().0.round() as i64,
                        "valve_closing_degree": (100 - demand_limit.from().0.round() as i64),
                        "temperature_accuracy": json_no_fraction_if_zero(-temperature_accuracy),
                    })],
                    false,
                )
                .await?;

                Ok(true)
            }
        }
    }

    pub async fn set_valve_opening_position_sonoff(&self, device_id: &str, value: Percent) -> anyhow::Result<bool> {
        let opened_percentage = (value.0.round() as i64).clamp(0, 100);
        let closing_percentage = 100 - opened_percentage;

        let payloads = if opened_percentage > 0 {
            vec![
                json!({
                    "valve_opening_degree": opened_percentage,
                    "valve_closing_degree": closing_percentage,
                }),
                json!({
                    "system_mode": "heat",
                    "occupied_heating_setpoint": 25,
                }),
            ]
        } else {
            vec![
                json!({
                    "valve_opening_degree": opened_percentage,
                    "valve_closing_degree": closing_percentage,
                }),
                json!({
                    "system_mode": "heat",
                    "occupied_heating_setpoint": 7,
                }),
            ]
        };

        self.send_message(device_id, payloads, false).await?;

        Ok(true)
    }

    pub async fn set_power_state(&self, device_id: &str, power_on: bool) -> anyhow::Result<bool> {
        let power_state = if power_on { "ON" } else { "OFF" };

        self.send_message(
            device_id,
            vec![json!({
                "state": power_state,
            })],
            true,
        )
        .await?;

        Ok(true)
    }

    async fn send_message(
        &self,
        device_id: &str,
        payloads: Vec<serde_json::Value>,
        optimistic: bool,
    ) -> anyhow::Result<()> {
        self.sender.send(device_id, payloads, optimistic).await?;

        CommandMetric::Executed {
            device_id: device_id.to_string(),
            system: CommandTargetSystem::Z2M,
        }
        .record();

        Ok(())
    }
}

fn json_no_fraction_if_zero(value: f64) -> serde_json::Value {
    if value.fract() == 0.0 {
        serde_json::json!(value as i64)
    } else {
        serde_json::json!(value)
    }
}

fn round_to_one_decimal_place(value: f64) -> f64 {
    (value * 10.0).round() / 10.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn z2m_topic_command_topic_uses_device_id_only() {
        let topic = Z2mTopic::new("/kitchen/light/");
        assert_eq!(topic.command_topic(), "kitchen/light/set");
    }

    #[test]
    fn z2m_topic_from_state_topic_extracts_device_id() {
        let topic = Z2mTopic::from_topic("living_room/sensor").expect("valid state topic");
        assert_eq!(topic.device_id(), "living_room/sensor");
    }

    #[test]
    fn z2m_topic_from_set_topic_extracts_device_id() {
        let topic = Z2mTopic::from_topic("living_room/sensor/set").expect("valid set topic");
        assert_eq!(topic.device_id(), "living_room/sensor");
    }

    #[test]
    fn z2m_topic_from_get_topic_extracts_device_id() {
        let topic = Z2mTopic::from_topic("living_room/sensor/get").expect("valid get topic");
        assert_eq!(topic.device_id(), "living_room/sensor");
    }

    #[test]
    fn z2m_topic_is_command_is_static() {
        assert!(Z2mTopic::is_command("living_room/sensor/set"));
        assert!(!Z2mTopic::is_command("living_room/sensor"));
    }

    #[test]
    fn z2m_topic_is_state_update_is_static() {
        assert!(Z2mTopic::is_state_update("living_room/sensor"));
        assert!(!Z2mTopic::is_state_update("living_room/sensor/set"));
    }

    #[test]
    fn test_round_to_one_decimal_place() {
        assert_eq!(round_to_one_decimal_place(1.234), 1.2);
        assert_eq!(round_to_one_decimal_place(1.25), 1.3);
        assert_eq!(round_to_one_decimal_place(1.0), 1.0);

        //0.3999999999999986
        let rounding_error = 19.0 - 18.6;
        assert!(rounding_error != 0.4);
        assert_eq!(round_to_one_decimal_place(rounding_error), 0.4);
    }
}
