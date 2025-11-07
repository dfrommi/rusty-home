use derive_more::derive::AsRef;
use std::fmt::Display;

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, AsRef)]
pub struct Lux(pub i64);

impl Display for Lux {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} lux", self.0)
    }
}

impl From<&Lux> for f64 {
    fn from(value: &Lux) -> Self {
        value.0 as f64
    }
}

impl From<f64> for Lux {
    fn from(value: f64) -> Self {
        Self(value as i64)
    }
}
