use crate::core::{ValueObject, time::DateTime};

use super::DataFrame;

pub trait Estimatable: ValueObject {
    fn interpolate(&self, at: DateTime, df: &DataFrame<Self::ValueType>)
    -> Option<Self::ValueType>;
}

pub mod algo {
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

        if prev.timestamp == at {
            return Some(prev.value.clone());
        } else if next.timestamp == at {
            return Some(next.value.clone());
        }

        let prev_time: f64 = prev.timestamp.into();
        let next_time: f64 = next.timestamp.into();
        let at_time: f64 = at.into();

        let prev_value: f64 = (&prev.value).into();
        let next_value: f64 = (&next.value).into();

        let interpolated_value = prev_value
            + (next_value - prev_value) * (at_time - prev_time) / (next_time - prev_time);

        Some(interpolated_value.into())
    }
}
