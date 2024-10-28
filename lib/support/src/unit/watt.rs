use derive_more::derive::AsRef;
use std::fmt::Display;

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, AsRef)]
pub struct Watt(pub f64);

impl Display for Watt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} W", self.0)
    }
}
