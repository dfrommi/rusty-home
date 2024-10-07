use std::{f64, fmt::Display};

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub enum OpenedState {
    Opened,
    Closed,
}

impl OpenedState {
    pub fn any(values: &[Self]) -> Self {
        if values.iter().any(|&state| state == OpenedState::Opened) {
            OpenedState::Opened
        } else {
            OpenedState::Closed
        }
    }

    pub fn is_opened(&self) -> bool {
        self == &Self::Opened
    }
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
