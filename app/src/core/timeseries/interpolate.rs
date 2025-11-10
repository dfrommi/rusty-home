use crate::core::time::DateTime;
use crate::core::timeseries::DataPoint;
use crate::port::ValueObject;
use anyhow::Result;

use super::DataFrame;

pub trait Interpolator<T>: Copy {
    fn interpolate(&self, at: DateTime, prev: &DataPoint<T>, next: &DataPoint<T>) -> Result<T>;
    fn interpolate_df(&self, at: DateTime, df: &DataFrame<T>) -> Result<Option<T>>;
}

#[derive(Copy, Clone)]
pub struct LastSeenInterpolator;
impl<T: Clone> Interpolator<T> for LastSeenInterpolator {
    fn interpolate(&self, _at: DateTime, prev: &DataPoint<T>, _next: &DataPoint<T>) -> Result<T> {
        Ok(prev.value.clone())
    }

    fn interpolate_df(&self, at: DateTime, df: &DataFrame<T>) -> Result<Option<T>> {
        Ok(df.prev_or_at(at).map(|dp| dp.value.clone()))
    }
}

#[derive(Copy, Clone)]
pub struct LinearInterpolator;
impl<T> Interpolator<T> for LinearInterpolator
where
    T: From<f64> + Clone,
    for<'a> &'a T: Into<f64>,
{
    fn interpolate(&self, at: DateTime, prev: &DataPoint<T>, next: &DataPoint<T>) -> Result<T> {
        algo::linear_dp(at, prev, next)
    }

    fn interpolate_df(&self, at: DateTime, df: &DataFrame<T>) -> Result<Option<T>> {
        let (prev, next) = match (df.prev_or_at(at), df.next(at)) {
            (Some(prev), Some(next)) => (prev, next),
            _ => return Ok(None),
        };

        self.interpolate(at, prev, next).map(Some)
    }
}

pub trait Estimatable: ValueObject {
    fn interpolate(&self, at: DateTime, df: &DataFrame<Self::ValueType>) -> Option<Self::ValueType>;
}

pub mod algo {
    use crate::core::timeseries::DataPoint;

    use super::*;

    pub fn last_seen<T>(at: DateTime, df: &DataFrame<T>) -> Option<T>
    where
        T: Clone,
    {
        df.prev_or_at(at).map(|dp| dp.value.clone())
    }

    //linear interpolation or last seen
    pub fn linear<T>(at: DateTime, df: &DataFrame<T>) -> Option<T>
    where
        T: From<f64> + Clone,
        for<'a> &'a T: Into<f64>,
    {
        let (prev, next) = match (df.prev_or_at(at), df.next(at)) {
            (Some(prev), Some(next)) => (prev, next),
            _ => return None,
        };

        linear_dp(at, prev, next).ok()
    }

    pub fn linear_dp<T>(at: DateTime, prev: &DataPoint<T>, next: &DataPoint<T>) -> Result<T>
    where
        T: From<f64> + Clone,
        for<'a> &'a T: Into<f64>,
    {
        if prev.timestamp > at || next.timestamp < at || prev.timestamp > next.timestamp {
            anyhow::bail!("Cannot interpolate: timestamps out of order");
        }

        if prev.timestamp == at {
            return Ok(prev.value.clone());
        } else if next.timestamp == at {
            return Ok(next.value.clone());
        }

        let prev_time: f64 = prev.timestamp.into();
        let next_time: f64 = next.timestamp.into();
        let at_time: f64 = at.into();

        let prev_value: f64 = (&prev.value).into();
        let next_value: f64 = (&next.value).into();

        let interpolated_value =
            prev_value + (next_value - prev_value) * (at_time - prev_time) / (next_time - prev_time);

        Ok(interpolated_value.into())
    }
}
