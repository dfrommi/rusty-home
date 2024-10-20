use std::{f64, fmt::Display, hash::Hasher};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct DegreeCelsius(pub f64);

impl From<&DegreeCelsius> for f64 {
    fn from(value: &DegreeCelsius) -> Self {
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

impl std::ops::Sub for &DegreeCelsius {
    type Output = DegreeCelsius;

    fn sub(self, rhs: Self) -> Self::Output {
        DegreeCelsius(self.0 - rhs.0)
    }
}
