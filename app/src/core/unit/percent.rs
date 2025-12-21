use std::{
    f64,
    fmt::Display,
    ops::{Mul, Sub},
};

use derive_more::derive::AsRef;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, AsRef, Serialize, Deserialize)]
pub struct Percent(pub f64);

impl Percent {
    pub fn clamp(self) -> Self {
        Self(self.0.clamp(0.0, 100.0))
    }

    pub fn abs(self) -> Self {
        Self(self.0.abs())
    }
}

impl From<&Percent> for f64 {
    fn from(value: &Percent) -> Self {
        value.0
    }
}

impl From<f64> for Percent {
    fn from(value: f64) -> Self {
        Self(value)
    }
}

impl From<Percent> for f64 {
    fn from(value: Percent) -> Self {
        value.0
    }
}

impl Display for Percent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:.2} %", self.0)
    }
}

impl Sub for Percent {
    type Output = Percent;

    fn sub(self, rhs: Self) -> Self::Output {
        Percent(self.0 - rhs.0)
    }
}

impl Mul<f64> for Percent {
    type Output = Percent;

    fn mul(self, rhs: f64) -> Self::Output {
        Percent(self.0 * rhs)
    }
}

impl Mul<Percent> for f64 {
    type Output = Percent;

    fn mul(self, rhs: Percent) -> Self::Output {
        Percent(self * rhs.0)
    }
}
