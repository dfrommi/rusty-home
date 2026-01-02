#![allow(dead_code)]

use crate::{
    core::{
        time::{DateTime, Duration},
        timeseries::{DataFrame, DataPoint, interpolate::Interpolator},
        unit::{Probability, p},
    },
    t,
};

pub trait DataFrameStatsExt<T: Clone> {
    fn weighted_aged_sum(&self, tau: Duration, interpolator: impl Interpolator<T>) -> f64;
    fn weighted_aged_mean(&self, tau: Duration, interpolator: impl Interpolator<T>) -> f64;
    fn average(&self) -> f64;
}

impl<T> DataFrameStatsExt<T> for DataFrame<T>
where
    T: Into<f64> + Clone,
{
    fn weighted_aged_sum(&self, tau: Duration, interpolator: impl Interpolator<T>) -> f64 {
        //scale integral to hours
        age_weighted_sum_and_count(self, tau, interpolator).0 / 3600.0
    }

    fn weighted_aged_mean(&self, tau: Duration, interpolator: impl Interpolator<T>) -> f64 {
        let (sum, total_weight) = age_weighted_sum_and_count(self, tau, interpolator);
        if total_weight != 0.0 { sum / total_weight } else { 0.0 }
    }

    fn average(&self) -> f64 {
        let values = self
            .current_and_next()
            .into_iter()
            .map(|(dp1, dp2)| {
                let dp2 = match dp2 {
                    Some(dp2) => dp2,
                    None => &DataPoint::new(dp1.value.clone(), t!(now)),
                };

                let value1: f64 = dp1.value.clone().into();
                let value2: f64 = dp2.value.clone().into();
                let weight = dp2.timestamp.elapsed_since(dp1.timestamp).as_secs_f64();
                let avg = weight * (value1 + value2) / 2.0;
                (avg, weight)
            })
            .collect::<Vec<_>>();

        let total_weight: f64 = values.iter().map(|(_, w)| *w).sum();
        let total_value: f64 = values.iter().map(|(v, _)| *v).sum();
        if total_weight != 0.0 {
            total_value / total_weight
        } else {
            0.0
        }
    }
}

fn age_weighted_sum_and_count<T>(df: &DataFrame<T>, tau: Duration, interpolator: impl Interpolator<T>) -> (f64, f64)
where
    T: Into<f64> + Clone,
{
    let values = df
        .current_and_next()
        .into_iter()
        .map(|(dp1, dp2)| {
            let dp2 = match dp2 {
                Some(dp2) => dp2,
                None => &DataPoint::new(dp1.value.clone(), t!(now)),
            };

            assert!(dp1.timestamp <= dp2.timestamp);

            let age_factor = tau.as_secs_f64()
                * (exp_decay_since(dp2.timestamp, tau.clone()) - exp_decay_since(dp1.timestamp, tau.clone()));

            let value: T = interpolator
                .interpolate(DateTime::midpoint(&dp1.timestamp, &dp2.timestamp), dp1, dp2)
                //this should really never ever happen as "at" is guaranteed to be between dp1 and dp2
                .unwrap_or(dp1.value.clone());

            let value: f64 = value.into();
            (value, age_factor)
        })
        .collect::<Vec<_>>();

    let sum: f64 = values.iter().map(|(v, w)| v * w).sum();
    let weights_sum: f64 = values.iter().map(|(_, w)| *w).sum();

    (sum, weights_sum)
}

fn sigmoid<T: Into<f64>>(x: T) -> Probability {
    let x: f64 = x.into();
    p(1.0 / (1.0 + (-x).exp()))
}

//inverse of sigmoid: sigmoid(logit(p)) = p
//use for prior of sigmoid functions
fn logit(p: Probability) -> f64 {
    let p: f64 = f64::from(p).clamp(1e-9, 1.0 - 1e-9); //avoid NaN
    (p / (1.0 - p)).ln()
}

fn exp_decay_since(ts: DateTime, tau: Duration) -> f64 {
    let dt = ts.elapsed().as_secs_f64();
    let tau = tau.as_secs_f64();
    (-dt / tau).exp()
}

// fn tau_from_half_life(t_half: f64) -> f64 {
//     t_half / std::f64::consts::LN_2
// }
// fn exp_decay<T>(x: T, m: T, tau: f64) -> f64
// where
//     T: Into<f64>,
// {
//     let x: f64 = x.into();
//     let m: f64 = m.into();
//     (-(x - m) / tau).exp()
// }

// fn sigmoid<T>(x: T, m: T, tau: f64) -> f64
// where
//     T: Into<f64>,
// {
//     1.0 / (1.0 + exp_decay(x, m, tau))
// }

//
//
//
// fn age_weighted_value<T>(dp: &DataPoint<T>, tau: Duration) -> f64
// where
//     T: Into<f64> + Clone,
// {
//     let age_factor = aging_factor(dp.timestamp, tau);
//     let value: f64 = dp.value.clone().into();
//     value * age_factor
// }

//higher values lead to higher probability
#[derive(Debug)]
pub struct Sigmoid<T>
where
    T: Into<f64> + From<f64>,
{
    center: f64,
    k: f64,
    _marker: std::marker::PhantomData<T>,
}

impl<T> Sigmoid<T>
where
    T: Into<f64> + From<f64>,
{
    pub fn from_example(c1: (Probability, T), c2: (Probability, T)) -> Self {
        let x1: f64 = c1.1.into();
        let x2: f64 = c2.1.into();

        let center = (x1 + x2) / 2.0;
        let k = (x1 - x2) / (logit(c1.0) - logit(c2.0));

        Self {
            center,
            k,
            _marker: std::marker::PhantomData,
        }
    }

    pub fn around(center: T, width_p80: T) -> Self {
        let center: f64 = center.into();
        let width: f64 = width_p80.into();
        let k = width / (logit(p(0.9)) - logit(p(0.1)));

        Self {
            center,
            k,
            _marker: std::marker::PhantomData,
        }
    }

    pub fn eval(&self, x: T) -> Probability {
        let x: f64 = x.into();
        sigmoid((x - self.center) / self.k)
    }

    pub fn inverse(&self, p: Probability) -> T {
        let x = self.k * logit(p) + self.center;
        T::from(x)
    }
}

impl<T> Default for Sigmoid<T>
where
    T: Into<f64> + From<f64>,
{
    fn default() -> Self {
        Self {
            center: 0.0,
            k: 1.0,
            _marker: std::marker::PhantomData,
        }
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

#[cfg(test)]
mod tests {
    use crate::{
        core::{
            timeseries::{
                DataPoint,
                interpolate::{LastSeenInterpolator, LinearInterpolator},
            },
            unit::Percent,
        },
        t,
    };

    use super::*;

    #[test]
    fn test_sigmoid_logit_relation() {
        assert_eq!(sigmoid(logit(p(0.8))), p(0.8));
    }

    #[test]
    fn test_sigmoid_default_inverse() {
        let sigmoid = Sigmoid::default();
        assert_approx(sigmoid.eval(0.0), 0.5);
        assert_approx(sigmoid.inverse(p(0.5)), 0.0);
    }

    #[test]
    fn test_sigmoid_inverse() {
        let sigmoid = Sigmoid::from_example((p(0.9), 40.0), (p(0.1), 10.0));
        assert_approx(sigmoid.inverse(sigmoid.eval(40.0)), 40.0);
        assert_approx(sigmoid.inverse(sigmoid.eval(30.0)), 30.0);
        assert_approx(sigmoid.inverse(sigmoid.eval(20.0)), 20.0);
    }

    #[test]
    fn test_sigmoid_mapping() {
        let sigmoid = Sigmoid::from_example((p(0.9), 80.0), (p(0.1), 20.0));
        assert_approx(sigmoid.eval(80.0), 0.9);
        assert_approx(sigmoid.eval(20.0), 0.1);
        assert_approx(sigmoid.eval(50.0), 0.5);
    }

    #[test]
    fn test_sigmoid_around() {
        let sigmoid = Sigmoid::around(50.0, 80.0);

        assert_approx(sigmoid.eval(90.0), 0.9);
        assert_approx(sigmoid.eval(10.0), 0.1);
        assert_approx(sigmoid.eval(50.0), 0.5);
    }

    fn assert_approx<S: Into<f64>, T: Into<f64>>(actual: S, expected: T) {
        let actual: f64 = actual.into();
        let expected: f64 = expected.into();
        let diff = (actual - expected).abs();
        assert!(diff < 1e-6, "Expected {} to be approx. {}", actual, expected,);
    }

    #[test]
    fn test_average() {
        let df = DataFrame::new(vec![
            DataPoint::new(10.0, t!(20 minutes ago)),
            DataPoint::new(20.0, t!(10 minutes ago)),
            DataPoint::new(30.0, t!(now)),
        ]);

        let avg = df.average();
        println!("Average: {}", avg);
        assert!(avg == 20.0);
    }

    #[test]
    fn test_weighted_aged_mean() {
        let df = DataFrame::new(vec![
            DataPoint::new(Percent(10.0), t!(30 minutes ago)),
            DataPoint::new(Percent(20.0), t!(20 minutes ago)),
            DataPoint::new(Percent(30.0), t!(10 minutes ago)),
        ]);

        let wam = df.weighted_aged_mean(t!(15 minutes), LastSeenInterpolator);
        assert!(wam > 20.0 && wam < 25.0);
    }

    #[test]
    fn test_weighted_aged_sum() {
        let df = DataFrame::new(vec![
            DataPoint::new(Percent(10.0), t!(60 minutes ago)),
            DataPoint::new(Percent(20.0), t!(40 minutes ago)),
            DataPoint::new(Percent(30.0), t!(20 minutes ago)),
        ]);

        let was = df.weighted_aged_sum(t!(30 minutes), LinearInterpolator);
        assert!(was > 10.0 && was < 12.0);
    }
}
