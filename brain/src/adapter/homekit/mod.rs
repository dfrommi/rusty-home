mod command;
mod state;

use api::state::unit::FanSpeed;
pub use command::process_commands;
pub use state::export_state;
use support::unit::Percent;

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
