use std::fmt::Display;

use derive_more::AsRef;

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, AsRef)]
pub struct GramPerCubicMeter(pub f64);

impl From<&GramPerCubicMeter> for f64 {
    fn from(value: &GramPerCubicMeter) -> Self {
        value.0
    }
}

impl From<f64> for GramPerCubicMeter {
    fn from(value: f64) -> Self {
        Self(value)
    }
}

impl From<GramPerCubicMeter> for f64 {
    fn from(value: GramPerCubicMeter) -> Self {
        value.0
    }
}

impl Display for GramPerCubicMeter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} g/m³", self.0)
    }
}
