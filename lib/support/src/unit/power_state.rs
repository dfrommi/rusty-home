use std::{f64, fmt::Display};

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub enum PowerState {
    On,
    Off,
}

impl PowerState {
    pub fn is_on(self) -> bool {
        Self::On == self
    }
}

impl From<&PowerState> for f64 {
    fn from(value: &PowerState) -> Self {
        match value {
            PowerState::On => 1.0,
            PowerState::Off => 0.0,
        }
    }
}

impl From<f64> for PowerState {
    fn from(value: f64) -> Self {
        if value > 0.0 {
            Self::On
        } else {
            Self::Off
        }
    }
}

impl Display for PowerState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PowerState::On => write!(f, "on"),
            PowerState::Off => write!(f, "off"),
        }
    }
}
