use std::fmt::Display;

use derive_more::derive::AsRef;
use serde::{Deserialize, Serialize};

use crate::core::{time::Duration, unit::RateOfChange};

#[derive(Debug, Clone, Copy, Default, PartialEq, PartialOrd, AsRef, Serialize, Deserialize)]
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

impl From<DegreeCelsius> for f64 {
    fn from(value: DegreeCelsius) -> Self {
        value.0
    }
}

impl Display for DegreeCelsius {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:.2} °C", self.0)
    }
}

impl std::ops::Add for DegreeCelsius {
    type Output = DegreeCelsius;

    fn add(self, rhs: Self) -> Self::Output {
        DegreeCelsius(self.0 + rhs.0)
    }
}

impl std::ops::Add for &DegreeCelsius {
    type Output = DegreeCelsius;

    fn add(self, rhs: Self) -> Self::Output {
        *self + *rhs
    }
}

impl std::ops::Sub for DegreeCelsius {
    type Output = DegreeCelsius;

    fn sub(self, rhs: Self) -> Self::Output {
        DegreeCelsius(self.0 - rhs.0)
    }
}

impl std::ops::Sub for &DegreeCelsius {
    type Output = DegreeCelsius;

    fn sub(self, rhs: Self) -> Self::Output {
        *self - *rhs
    }
}

impl std::ops::Mul<f64> for DegreeCelsius {
    type Output = DegreeCelsius;

    fn mul(self, rhs: f64) -> Self::Output {
        DegreeCelsius(self.0 * rhs)
    }
}

impl std::ops::Mul<DegreeCelsius> for f64 {
    type Output = DegreeCelsius;

    fn mul(self, rhs: DegreeCelsius) -> Self::Output {
        DegreeCelsius(self * rhs.0)
    }
}

impl std::ops::Div<f64> for DegreeCelsius {
    type Output = DegreeCelsius;

    fn div(self, rhs: f64) -> Self::Output {
        DegreeCelsius(self.0 / rhs)
    }
}

impl std::ops::Div for DegreeCelsius {
    type Output = f64;

    fn div(self, rhs: Self) -> Self::Output {
        self.0 / rhs.0
    }
}

impl std::ops::Neg for DegreeCelsius {
    type Output = DegreeCelsius;

    fn neg(self) -> Self::Output {
        DegreeCelsius(-self.0)
    }
}

impl std::ops::Div<Duration> for DegreeCelsius {
    type Output = RateOfChange<DegreeCelsius>;

    fn div(self, rhs: Duration) -> Self::Output {
        RateOfChange::new(self, rhs)
    }
}
