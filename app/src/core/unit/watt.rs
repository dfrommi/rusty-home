use derive_more::derive::AsRef;
use std::fmt::Display;

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, AsRef)]
pub struct Watt(pub f64);

impl Display for Watt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} W", self.0)
    }
}

impl From<&Watt> for f64 {
    fn from(value: &Watt) -> Self {
        value.0
    }
}

impl From<f64> for Watt {
    fn from(value: f64) -> Self {
        Self(value)
    }
}
