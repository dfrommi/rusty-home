use std::{f64, fmt::Display};

use derive_more::derive::AsRef;

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, AsRef)]
pub struct Percent(pub f64);

//Made-up unit to represent percentage-usage over time.
//100% for one hour is 1 PercentHour
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, AsRef)]
pub struct PercentHour(pub f64);

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

impl Display for Percent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} %", self.0)
    }
}
