use std::{f64, fmt::Display};

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub enum PresentState {
    Present,
    Absent,
}

impl Display for PresentState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PresentState::Present => write!(f, "presend"),
            PresentState::Absent => write!(f, "not present"),
        }
    }
}
