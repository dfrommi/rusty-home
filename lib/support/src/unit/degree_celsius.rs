use std::{f64, fmt::Display};

#[derive(Debug, Clone, Copy, Default, PartialEq, PartialOrd)]
pub struct DegreeCelsius(pub f64);

impl From<&DegreeCelsius> for f64 {
    fn from(value: &DegreeCelsius) -> Self {
        value.0
    }
}

impl From<DegreeCelsius> for f64 {
    fn from(value: DegreeCelsius) -> Self {
        value.0
    }
}

impl From<f64> for DegreeCelsius {
    fn from(value: f64) -> Self {
        Self(value)
    }
}

impl Display for DegreeCelsius {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} °C", self.0)
    }
}
