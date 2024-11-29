use std::fmt::Display;

use derive_more::derive::AsRef;

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, AsRef)]
pub struct KiloCubicMeter(pub f64);

impl From<&KiloCubicMeter> for f64 {
    fn from(value: &KiloCubicMeter) -> Self {
        value.0
    }
}

impl From<f64> for KiloCubicMeter {
    fn from(value: f64) -> Self {
        Self(value)
    }
}

impl Display for KiloCubicMeter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} m³", self.0)
    }
}
