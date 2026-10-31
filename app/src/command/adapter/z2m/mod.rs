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
        self.sender
            .send_transient(set_topic, heating_payload(state).to_string())
            .await?;

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

fn heating_payload(state: HeatingTargetState) -> serde_json::Value {
    match state {
        HeatingTargetState::Off => json!({
            "system_mode": "off",
            "occupied_heating_setpoint": 7,
            "valve_opening_degree": 0,
            "valve_closing_degree": 100,
            "temperature_accuracy": -1,
        }),
        HeatingTargetState::Heat {
            target_temperature,
            demand,
        } => {
            let temperature_accuracy =
                round_to_one_decimal((target_temperature.to().0 - target_temperature.from().0).clamp(0.2, 1.0));

            json!({
                "system_mode": "heat",
                "occupied_heating_setpoint": json_no_fraction_if_zero(target_temperature.to().0),
                "valve_opening_degree": demand.0.round() as i64,
                "valve_closing_degree": 100,
                "temperature_accuracy": json_no_fraction_if_zero(-temperature_accuracy),
            })
        }
    }
}

fn json_no_fraction_if_zero(value: f64) -> serde_json::Value {
    if value.fract() == 0.0 {
        serde_json::json!(value as i64)
    } else {
        serde_json::json!(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{
        range::Range,
        unit::{DegreeCelsius, Percent},
    };

    #[test]
    fn heat_commands_preserve_the_on_amplitude_and_force_full_closing() {
        for (demand, opening) in [(0.0, 0), (35.6, 36), (100.0, 100)] {
            let state = HeatingTargetState::Heat {
                target_temperature: Range::new(DegreeCelsius(19.6), DegreeCelsius(20.0)),
                demand: Percent(demand),
            };

            assert_eq!(
                heating_payload(state),
                json!({
                    "system_mode": "heat",
                    "occupied_heating_setpoint": 20,
                    "valve_opening_degree": opening,
                    "valve_closing_degree": 100,
                    "temperature_accuracy": -0.4,
                })
            );
        }
    }

    #[test]
    fn off_commands_force_full_closing_and_zero_opening() {
        assert_eq!(
            heating_payload(HeatingTargetState::Off),
            json!({
                "system_mode": "off",
                "occupied_heating_setpoint": 7,
                "valve_opening_degree": 0,
                "valve_closing_degree": 100,
                "temperature_accuracy": -1,
            })
        );
    }
}
