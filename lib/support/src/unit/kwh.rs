use std::fmt::Display;

use derive_more::derive::AsRef;

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, AsRef)]
pub struct KiloWattHours(pub f64);

impl Display for KiloWattHours {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} kWh", self.0)
    }
}
