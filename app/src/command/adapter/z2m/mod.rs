mod config;
mod sync;

pub use sync::Z2mSensorSyncRunner;

use crate::{
    command::{
        Command, CommandTarget, HeatingTargetState,
        adapter::{
            CommandExecutor,
            metrics::{CommandMetric, CommandTargetSystem},
        },
    },
    core::math::round_to_one_decimal,
};
use infrastructure::MqttSender;
use serde_json::json;

#[derive(Debug, Clone)]
pub enum Z2mCommandTarget {
    SonoffThermostat(&'static str),
    PowerPlug(&'static str),
}

pub struct Z2mCommandExecutor {
    config: Vec<(CommandTarget, Z2mCommandTarget)>,
    sender: MqttSender,
}

impl Z2mCommandExecutor {
    pub fn new(mqtt_sender: MqttSender) -> Self {
        let config = config::default_z2m_command_config();
        Self {
            config,
            sender: mqtt_sender,
        }
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

        let device_id = match (command, z2m_target) {
            (Command::SetPower { power_on, .. }, Z2mCommandTarget::PowerPlug(device_id)) => {
                self.set_power_state(device_id, *power_on).await?;
                device_id
            }
            (Command::SetHeating { target_state, .. }, Z2mCommandTarget::SonoffThermostat(device_id)) => {
                self.set_sonoff_heating(device_id, target_state.clone()).await?;
                device_id
            }
            (_, z2m_target) => {
                anyhow::bail!("Mismatch between command and Z2M target {:?}", z2m_target)
            }
        };

        CommandMetric::Executed {
            device_id: device_id.to_string(),
            system: CommandTargetSystem::Z2M,
        }
        .record();

        Ok(true)
    }
}

impl Z2mCommandExecutor {
    pub async fn set_sonoff_heating(&self, device_id: &str, state: HeatingTargetState) -> anyhow::Result<()> {
        let set_topic = format!("{}/set", device_id);

        match state {
            HeatingTargetState::Off => {
                self.sender
                    .send_transient(
                        set_topic,
                        json!({
                            "system_mode": "off",
                            "occupied_heating_setpoint": 7,
                            "valve_opening_degree": 0,
                            "valve_closing_degree": 100,
                            "temperature_accuracy": -1,
                        })
                        .to_string(),
                    )
                    .await?;

                Ok(())
            }
            HeatingTargetState::Heat {
                target_temperature,
                demand_limit,
            } => {
                let temperature_accuracy =
                    round_to_one_decimal((target_temperature.to().0 - target_temperature.from().0).clamp(0.2, 1.0));

                self.sender
                    .send_transient(
                        set_topic,
                        json!({
                            "system_mode": "heat",
                            "occupied_heating_setpoint": json_no_fraction_if_zero(target_temperature.to().0),
                            "valve_opening_degree": demand_limit.to().0.round() as i64,
                            "valve_closing_degree": (100 - demand_limit.from().0.round() as i64),
                            "temperature_accuracy": json_no_fraction_if_zero(-temperature_accuracy),
                        })
                        .to_string(),
                    )
                    .await?;

                Ok(())
            }
        }
    }

    pub async fn set_power_state(&self, device_id: &str, power_on: bool) -> anyhow::Result<()> {
        let set_topic = format!("{}/set", device_id);
        let power_state = if power_on { "ON" } else { "OFF" };

        self.sender
            .send_transient(
                set_topic,
                json!({
                     "state": power_state,
                })
                .to_string(),
            )
            .await?;

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
