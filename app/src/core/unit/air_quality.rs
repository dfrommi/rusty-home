use std::fmt::Display;

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct AllergenIndexValue(pub i64);

impl Display for AllergenIndexValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} AI", self.0)
    }
}

impl From<&AllergenIndexValue> for f64 {
    fn from(value: &AllergenIndexValue) -> Self {
        value.0 as f64
    }
}

impl From<f64> for AllergenIndexValue {
    fn from(value: f64) -> Self {
        Self(value as i64)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct MicrogramsPerCubicMeter(pub f64);

impl Display for MicrogramsPerCubicMeter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} µg/m³", self.0)
    }
}

impl From<&MicrogramsPerCubicMeter> for f64 {
    fn from(value: &MicrogramsPerCubicMeter) -> Self {
        value.0
    }
}

impl From<f64> for MicrogramsPerCubicMeter {
    fn from(value: f64) -> Self {
        Self(value)
    }
}
