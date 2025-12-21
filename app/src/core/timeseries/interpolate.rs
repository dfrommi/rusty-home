use crate::core::time::DateTime;
use crate::core::timeseries::DataPoint;
use anyhow::Result;

use super::DataFrame;

pub trait Interpolator<T: Clone>: Copy {
    fn interpolate(&self, at: DateTime, prev: &DataPoint<T>, next: &DataPoint<T>) -> Result<T>;
    fn interpolate_df(&self, at: DateTime, df: &DataFrame<T>) -> Option<T>;
}

#[derive(Copy, Clone)]
pub struct LastSeenInterpolator;
impl<T: Clone> Interpolator<T> for LastSeenInterpolator {
    fn interpolate(&self, _at: DateTime, prev: &DataPoint<T>, _next: &DataPoint<T>) -> Result<T> {
        Ok(prev.value.clone())
    }

    fn interpolate_df(&self, at: DateTime, df: &DataFrame<T>) -> Option<T> {
        df.prev_or_at(at).map(|dp| dp.value.clone())
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
        assert_params_consistent(at, prev, next)?;
        Ok(linear_dp(at, prev, next))
    }

    fn interpolate_df(&self, at: DateTime, df: &DataFrame<T>) -> Option<T> {
        match (df.prev_or_at(at), df.next(at)) {
            (Some(prev), Some(next)) => Some(linear_dp(at, &prev, &next)),
            _ => return None,
        }
    }
}

#[derive(Copy, Clone)]
pub struct LinearOrLastSeenInterpolator;
impl<T> Interpolator<T> for LinearOrLastSeenInterpolator
where
    T: From<f64> + Clone,
    for<'a> &'a T: Into<f64>,
{
    fn interpolate(&self, at: DateTime, prev: &DataPoint<T>, next: &DataPoint<T>) -> Result<T> {
        assert_params_consistent(at, prev, next)?;
        LinearInterpolator.interpolate(at, prev, next)
    }

    fn interpolate_df(&self, at: DateTime, df: &DataFrame<T>) -> Option<T> {
        LinearInterpolator
            .interpolate_df(at, df)
            .or_else(|| LastSeenInterpolator.interpolate_df(at, df))
    }
}

fn assert_params_consistent<T>(at: DateTime, prev: &DataPoint<T>, next: &DataPoint<T>) -> Result<()> {
    if prev.timestamp > at {
        anyhow::bail!(
            "Cannot interpolate: prev timestamp {} is after requested timestamp {}",
            prev.timestamp,
            at
        );
    }
    if next.timestamp < at {
        anyhow::bail!(
            "Cannot interpolate: next timestamp {} is before requested timestamp {}",
            next.timestamp,
            at
        );
    }
    if prev.timestamp > next.timestamp {
        anyhow::bail!(
            "Cannot interpolate: prev timestamp {} is after next timestamp {}",
            prev.timestamp,
            next.timestamp
        );
    }

    Ok(())
}

fn linear_dp<T>(at: DateTime, prev: &DataPoint<T>, next: &DataPoint<T>) -> T
where
    T: From<f64> + Clone,
    for<'a> &'a T: Into<f64>,
{
    if prev.timestamp == at {
        return prev.value.clone();
    } else if next.timestamp == at {
        return next.value.clone();
    }

    let prev_time: f64 = prev.timestamp.into();
    let next_time: f64 = next.timestamp.into();
    let at_time: f64 = at.into();

    let prev_value: f64 = (&prev.value).into();
    let next_value: f64 = (&next.value).into();

    let interpolated_value = prev_value + (next_value - prev_value) * (at_time - prev_time) / (next_time - prev_time);

    interpolated_value.into()
}
