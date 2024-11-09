use support::{
    time::DateTime,
    unit::{DegreeCelsius, KiloWattHours, Percent, Watt},
};

use crate::adapter::persistence::DataPoint;

pub trait Interpolatable: Sized {
    fn interpolate(
        at: DateTime,
        prev: Option<&DataPoint<Self>>,
        next: Option<&DataPoint<Self>>,
    ) -> Option<Self>;
}

impl Interpolatable for bool {
    fn interpolate(
        at: DateTime,
        prev: Option<&DataPoint<bool>>,
        next: Option<&DataPoint<bool>>,
    ) -> Option<bool> {
        algo::last_seen(at, prev, next)
    }
}

impl Interpolatable for DegreeCelsius {
    fn interpolate(
        at: DateTime,
        prev: Option<&DataPoint<DegreeCelsius>>,
        next: Option<&DataPoint<DegreeCelsius>>,
    ) -> Option<DegreeCelsius> {
        algo::linear(at, prev, next)
    }
}

impl Interpolatable for Percent {
    fn interpolate(
        at: DateTime,
        prev: Option<&DataPoint<Percent>>,
        next: Option<&DataPoint<Percent>>,
    ) -> Option<Percent> {
        algo::linear(at, prev, next)
    }
}

impl Interpolatable for Watt {
    fn interpolate(
        at: DateTime,
        prev: Option<&DataPoint<Watt>>,
        next: Option<&DataPoint<Watt>>,
    ) -> Option<Watt> {
        algo::last_seen(at, prev, next)
    }
}

impl Interpolatable for KiloWattHours {
    fn interpolate(
        at: DateTime,
        prev: Option<&DataPoint<KiloWattHours>>,
        next: Option<&DataPoint<KiloWattHours>>,
    ) -> Option<KiloWattHours> {
        algo::linear(at, prev, next)
    }
}

pub mod algo {
    use super::*;

    pub fn last_seen<T>(
        _: DateTime,
        prev: Option<&DataPoint<T>>,
        _: Option<&DataPoint<T>>,
    ) -> Option<T>
    where
        T: Clone,
    {
        prev.map(|dp| dp.value.clone())
    }

    //linear interpolation or last seen
    pub fn linear<T>(
        at: DateTime,
        prev: Option<&DataPoint<T>>,
        next: Option<&DataPoint<T>>,
    ) -> Option<T>
    where
        T: From<f64> + Clone,
        for<'a> &'a T: Into<f64>,
    {
        match (prev, next) {
            (Some(prev), _) if prev.timestamp == at => Some(prev.value.clone()),
            (_, Some(next)) if next.timestamp == at => Some(next.value.clone()),

            (Some(prev), Some(next)) => {
                let prev_time: f64 = prev.timestamp.into();
                let next_time: f64 = next.timestamp.into();
                let at_time: f64 = at.into();

                let prev_value: f64 = (&prev.value).into();
                let next_value: f64 = (&next.value).into();

                let interpolated_value = prev_value
                    + (next_value - prev_value) * (at_time - prev_time) / (next_time - prev_time);

                Some(interpolated_value.into())
            }

            //Fallback to last_seen
            (Some(prev), None) => Some(prev.value.clone()),

            _ => None,
        }
    }
}
