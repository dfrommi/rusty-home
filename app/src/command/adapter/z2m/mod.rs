mod sync;

pub use sync::Z2mSensorSyncRunner;

use super::metrics::{CommandMetric, CommandTargetSystem};
use crate::{command::HeatingTargetState, core::math::round_to_one_decimal};
use infrastructure::MqttSender;
use serde_json::json;

pub struct Z2mCommandExecutor {
    sender: MqttSender,
}

impl Z2mCommandExecutor {
    pub fn new(mqtt_sender: MqttSender) -> Self {
        Self { sender: mqtt_sender }
    }

    fn record_executed(&self, device_id: &str) {
        CommandMetric::Executed {
            device_id: device_id.to_string(),
            system: CommandTargetSystem::Z2M,
        }
        .record();
    }
}

impl Z2mCommandExecutor {
    pub async fn set_heating(&self, device_id: &str, state: HeatingTargetState) -> anyhow::Result<()> {
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
            }
        }

        self.record_executed(device_id);
        Ok(())
    }

    pub async fn set_power(&self, device_id: &str, power_on: bool) -> anyhow::Result<()> {
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

        self.record_executed(device_id);
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
