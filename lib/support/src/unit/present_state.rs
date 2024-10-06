use std::{f64, fmt::Display};

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub enum PresentState {
    Present,
    Absent,
}

impl From<&PresentState> for f64 {
    fn from(value: &PresentState) -> Self {
        match value {
            PresentState::Present => 1.0,
            PresentState::Absent => 0.0,
        }
    }
}

impl From<f64> for PresentState {
    fn from(value: f64) -> Self {
        if value > 0.0 {
            Self::Present
        } else {
            Self::Absent
        }
    }
}

impl Display for PresentState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PresentState::Present => write!(f, "presend"),
            PresentState::Absent => write!(f, "not present"),
        }
    }
}
