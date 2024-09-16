use std::fmt::Display;

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct KiloWattHours(pub f64);

impl From<&KiloWattHours> for f64 {
    fn from(value: &KiloWattHours) -> Self {
        value.0
    }
}

impl From<f64> for KiloWattHours {
    fn from(value: f64) -> Self {
        Self(value)
    }
}

impl Display for KiloWattHours {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} kWh", self.0)
    }
}
