use std::fmt::Display;

use derive_more::derive::AsRef;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, AsRef, Serialize, Deserialize)]
pub struct GramsPerCubicMeter(pub f64);

impl From<&GramsPerCubicMeter> for f64 {
    fn from(value: &GramsPerCubicMeter) -> Self {
        value.0
    }
}

impl From<f64> for GramsPerCubicMeter {
    fn from(value: f64) -> Self {
        Self(value)
    }
}

impl Display for GramsPerCubicMeter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:.2} g/m³", self.0)
    }
}
