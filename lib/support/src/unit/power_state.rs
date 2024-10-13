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

impl Display for PowerState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PowerState::On => write!(f, "on"),
            PowerState::Off => write!(f, "off"),
        }
    }
}
