use std::{f64, fmt::Display};

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Percent(pub f64);

impl From<&Percent> for f64 {
    fn from(value: &Percent) -> Self {
        value.0
    }
}

impl From<Percent> for f64 {
    fn from(value: Percent) -> Self {
        value.0
    }
}

impl From<f64> for Percent {
    fn from(value: f64) -> Self {
        Self(value)
    }
}

impl Display for Percent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} %", self.0)
    }
}
