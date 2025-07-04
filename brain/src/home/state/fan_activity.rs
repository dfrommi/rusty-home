use std::fmt::Display;

use r#macro::Id;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Hash, Eq, PartialEq, Id)]
pub enum FanActivity {
    LivingRoomCeilingFan,
    BedroomCeilingFan,
}

#[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub enum FanAirflow {
    Off,
    Forward(FanSpeed),
    Reverse(FanSpeed),
}

#[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub enum FanSpeed {
    Silent,
    Low,
    Medium,
    High,
    Turbo,
}

impl Display for FanAirflow {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FanAirflow::Off => write!(f, "off"),
            FanAirflow::Forward(FanSpeed::Silent) => write!(f, "silent (fwd)"),
            FanAirflow::Forward(FanSpeed::Low) => write!(f, "low (fwd)"),
            FanAirflow::Forward(FanSpeed::Medium) => write!(f, "medium (fwd)"),
            FanAirflow::Forward(FanSpeed::High) => write!(f, "high (fwd)"),
            FanAirflow::Forward(FanSpeed::Turbo) => write!(f, "turbo (fwd)"),
            FanAirflow::Reverse(FanSpeed::Silent) => write!(f, "silent (rev)"),
            FanAirflow::Reverse(FanSpeed::Low) => write!(f, "low (rev)"),
            FanAirflow::Reverse(FanSpeed::Medium) => write!(f, "medium (rev)"),
            FanAirflow::Reverse(FanSpeed::High) => write!(f, "high (rev)"),
            FanAirflow::Reverse(FanSpeed::Turbo) => write!(f, "turbo (rev)"),
        }
    }
}
