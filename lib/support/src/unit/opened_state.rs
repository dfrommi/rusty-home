use std::{f64, fmt::Display};

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub enum OpenedState {
    Opened,
    Closed,
}

impl From<&OpenedState> for f64 {
    fn from(value: &OpenedState) -> Self {
        match value {
            OpenedState::Opened => 1.0,
            OpenedState::Closed => 0.0,
        }
    }
}

impl From<f64> for OpenedState {
    fn from(value: f64) -> Self {
        if value > 0.0 {
            Self::Opened
        } else {
            Self::Closed
        }
    }
}

impl Display for OpenedState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OpenedState::Opened => write!(f, "opened"),
            OpenedState::Closed => write!(f, "closed"),
        }
    }
}
