#![allow(dead_code)]

use crate::core::unit::{Probability, p};

//higher values lead to higher probability
pub struct Sigmoid<T>
where
    T: Into<f64> + From<f64>,
{
    center: f64,
    slope: f64,
    inverse: bool,
    _marker: std::marker::PhantomData<T>,
}

impl<T> Sigmoid<T>
where
    T: Into<f64> + From<f64>,
{
    pub fn new(center: T, slope: f64) -> Self {
        Self {
            center: center.into(),
            slope,
            inverse: false,
            _marker: std::marker::PhantomData,
        }
    }

    pub fn inv(mut self) -> Self {
        self.inverse = true;
        self
    }

    pub fn eval(&self, x: T) -> Probability {
        let x: f64 = x.into();
        let result = p(1.0 / (1.0 + (-self.slope * (x - self.center)).exp()));

        if self.inverse { result.inv() } else { result }
    }
}

//values decresing the further they are from mu -> optimal value distribution
pub struct Gauss<T>
where
    T: Into<f64> + From<f64> + Clone,
{
    mu: f64,
    sigma: f64,
    inverse: bool,
    _marker: std::marker::PhantomData<T>,
}

impl<T> Gauss<T>
where
    T: Into<f64> + From<f64> + Clone,
{
    pub fn new(mu: T, sigma: f64) -> Self {
        Self {
            mu: mu.into(),
            sigma,
            inverse: false,
            _marker: std::marker::PhantomData,
        }
    }

    pub fn inv(mut self) -> Self {
        self.inverse = true;
        self
    }

    pub fn eval(&self, x: T) -> Probability {
        let x: f64 = x.into();
        p((-0.5 * ((x - self.mu) / self.sigma).powi(2)).exp())
    }
}

// values in range [-1, 1]
// - `x = center` → 0
// - `x < center` → negative
// - `x > center` → positive
pub struct Tanh<T>
where
    T: Into<f64> + From<f64>,
{
    center: f64,
    scale: f64,
    _marker: std::marker::PhantomData<T>,
}

impl<T> Tanh<T>
where
    T: Into<f64> + From<f64>,
{
    pub fn new(center: T, scale: f64) -> Self {
        Self {
            center: center.into(),
            scale,
            _marker: std::marker::PhantomData,
        }
    }

    pub fn eval(&self, x: T) -> f64 {
        let x: f64 = x.into();
        ((x - self.center) * self.scale).tanh()
    }
}
