use std::fmt::Display;

use derive_more::derive::AsRef;

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, AsRef)]
pub struct HeatingUnit(pub f64);

impl From<&HeatingUnit> for f64 {
    fn from(value: &HeatingUnit) -> Self {
        value.0
    }
}

impl From<f64> for HeatingUnit {
    fn from(value: f64) -> Self {
        Self(value)
    }
}

impl Display for HeatingUnit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} VBE", self.0) //Verbrauchseinheit
    }
}
