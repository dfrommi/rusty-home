use std::fmt::Display;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub enum FanAirflow {
    Off,
    Forward(FanSpeed),
}

#[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub enum FanSpeed {
    Low,
    Medium,
    High,
}

impl FanAirflow {
    pub fn is_off(&self) -> bool {
        self == &FanAirflow::Off
    }

    pub fn is_on(&self) -> bool {
        !self.is_off()
    }
}

impl Display for FanAirflow {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FanAirflow::Off => write!(f, "off"),
            FanAirflow::Forward(FanSpeed::Low) => write!(f, "low"),
            FanAirflow::Forward(FanSpeed::Medium) => write!(f, "medium"),
            FanAirflow::Forward(FanSpeed::High) => write!(f, "high"),
        }
    }
}

impl From<&FanAirflow> for f64 {
    fn from(value: &FanAirflow) -> Self {
        match value {
            FanAirflow::Off => 0.0,
            FanAirflow::Forward(FanSpeed::Low) => 2.0,
            FanAirflow::Forward(FanSpeed::Medium) => 3.0,
            FanAirflow::Forward(FanSpeed::High) => 4.0,
        }
    }
}

impl From<f64> for FanAirflow {
    fn from(value: f64) -> Self {
        if value > 3.0 {
            FanAirflow::Forward(FanSpeed::High)
        } else if value > 2.0 {
            FanAirflow::Forward(FanSpeed::Medium)
        } else if value > 1.0 {
            FanAirflow::Forward(FanSpeed::Low)
        } else {
            FanAirflow::Off
        }
    }
}
