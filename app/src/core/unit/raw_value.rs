use derive_more::AsRef;
use serde::{Deserialize, Serialize};

///A vendor-specific unit not following standard units
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, AsRef, Serialize, Deserialize)]
pub struct RawValue(pub f64);

impl From<&RawValue> for f64 {
    fn from(val: &RawValue) -> Self {
        val.0
    }
}

impl From<RawValue> for f64 {
    fn from(val: RawValue) -> Self {
        val.0
    }
}

impl From<f64> for RawValue {
    fn from(val: f64) -> Self {
        RawValue(val)
    }
}

impl std::fmt::Display for RawValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
