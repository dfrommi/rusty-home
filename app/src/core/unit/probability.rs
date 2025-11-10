use std::{f64, fmt::Display};

pub fn p(value: f64) -> Probability {
    Probability(value)
}

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Probability(f64);

impl Probability {
    pub fn factor(&self) -> f64 {
        self.0
    }

    pub fn inv(&self) -> Self {
        Self(1.0 - self.0)
    }
}

impl From<Probability> for f64 {
    fn from(value: Probability) -> Self {
        value.0
    }
}

impl From<&Probability> for f64 {
    fn from(value: &Probability) -> Self {
        value.0
    }
}

impl From<f64> for Probability {
    fn from(value: f64) -> Self {
        Self(value)
    }
}

impl Display for Probability {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::ops::Mul<f64> for Probability {
    type Output = Probability;

    fn mul(self, rhs: f64) -> Self::Output {
        Probability(self.0 * rhs)
    }
}

impl std::ops::Mul<Probability> for f64 {
    type Output = Probability;

    fn mul(self, rhs: Probability) -> Self::Output {
        Probability((self * rhs.0).min(1.0))
    }
}

impl std::ops::Mul for Probability {
    type Output = Probability;

    fn mul(self, rhs: Self) -> Self::Output {
        Probability(self.0 * rhs.0)
    }
}

impl std::ops::Add for Probability {
    type Output = Probability;

    fn add(self, rhs: Self) -> Self::Output {
        Probability(self.0 + rhs.0)
    }
}
