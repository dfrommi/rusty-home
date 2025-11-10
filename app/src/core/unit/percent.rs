use std::{f64, fmt::Display};

use derive_more::derive::AsRef;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, AsRef, Serialize, Deserialize)]
pub struct Percent(pub f64);

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
