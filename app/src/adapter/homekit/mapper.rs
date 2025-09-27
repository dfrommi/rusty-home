use crate::{
    core::unit::{DegreeCelsius, Percent},
    home::state::FanSpeed,
};

use super::HomekitStateValue;

impl From<bool> for HomekitStateValue {
    fn from(val: bool) -> Self {
        HomekitStateValue(if val { "1".to_string() } else { "0".to_string() })
    }
}

impl TryInto<bool> for HomekitStateValue {
    type Error = anyhow::Error;

    fn try_into(self) -> Result<bool, Self::Error> {
        match self.0.as_str() {
            "0" => Ok(false),
            "1" => Ok(true),
            _ => anyhow::bail!("Error converting {} to bool", self.0),
        }
    }
}

impl From<Percent> for HomekitStateValue {
    fn from(val: Percent) -> Self {
        HomekitStateValue(val.0.to_string())
    }
}

impl TryInto<Percent> for HomekitStateValue {
    type Error = anyhow::Error;

    fn try_into(self) -> Result<Percent, Self::Error> {
        match self.0.parse() {
            Ok(v) => Ok(Percent(v)),
            Err(_) => anyhow::bail!("Error converting {} to Percent", self.0),
        }
    }
}

impl From<DegreeCelsius> for HomekitStateValue {
    fn from(val: DegreeCelsius) -> Self {
        HomekitStateValue(val.0.to_string())
    }
}

impl TryInto<DegreeCelsius> for HomekitStateValue {
    type Error = anyhow::Error;

    fn try_into(self) -> Result<DegreeCelsius, Self::Error> {
        match self.0.parse() {
            Ok(v) => Ok(DegreeCelsius(v)),
            Err(_) => anyhow::bail!("Error converting {} to DegreeCelsius", self.0),
        }
    }
}

impl From<FanSpeed> for HomekitStateValue {
    fn from(val: FanSpeed) -> Self {
        HomekitStateValue(
            match val {
                FanSpeed::Silent => "20.0",
                FanSpeed::Low => "40.0",
                FanSpeed::Medium => "60.0",
                FanSpeed::High => "80.0",
                FanSpeed::Turbo => "100.0",
            }
            .to_string(),
        )
    }
}

impl TryInto<FanSpeed> for HomekitStateValue {
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
