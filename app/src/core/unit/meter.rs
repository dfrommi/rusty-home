use std::fmt::Display;

use derive_more::derive::AsRef;

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, AsRef)]
pub struct Meter(pub f64);

impl From<&Meter> for f64 {
    fn from(value: &Meter) -> Self {
        value.0
    }
}

impl From<f64> for Meter {
    fn from(value: f64) -> Self {
        Self(value)
    }
}

impl From<Meter> for f64 {
    fn from(value: Meter) -> Self {
        value.0
    }
}

impl Display for Meter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} m", self.0)
    }
}
