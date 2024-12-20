use api::state::{HeatingDemand, Presence, RelativeHumidity, Temperature, TotalEnergyConsumption};
use support::{
    time::DateTime,
    unit::{DegreeCelsius, KiloWattHours, Percent},
    DataPoint,
};

pub trait Estimatable
where
    Self::Type: Clone,
{
    type Type;

    fn interpolate(
        &self,
        at: DateTime,
        prev: &DataPoint<Self::Type>,
        next: &DataPoint<Self::Type>,
    ) -> Self::Type;
}

impl Estimatable for Temperature {
    type Type = DegreeCelsius;

    fn interpolate(
        &self,
        at: DateTime,
        prev: &DataPoint<Self::Type>,
        next: &DataPoint<Self::Type>,
    ) -> Self::Type {
        algo::linear(at, prev, next)
    }
}

impl Estimatable for RelativeHumidity {
    type Type = Percent;

    fn interpolate(
        &self,
        at: DateTime,
        prev: &DataPoint<Self::Type>,
        next: &DataPoint<Self::Type>,
    ) -> Self::Type {
        algo::linear(at, prev, next)
    }
}

impl Estimatable for Presence {
    type Type = bool;

    fn interpolate(
        &self,
        at: DateTime,
        prev: &DataPoint<Self::Type>,
        next: &DataPoint<Self::Type>,
    ) -> Self::Type {
        algo::last_seen(at, prev, next)
    }
}

impl Estimatable for HeatingDemand {
    type Type = Percent;

    fn interpolate(
        &self,
        at: DateTime,
        prev: &DataPoint<Self::Type>,
        next: &DataPoint<Self::Type>,
    ) -> Self::Type {
        algo::last_seen(at, prev, next)
    }
}

impl Estimatable for TotalEnergyConsumption {
    type Type = KiloWattHours;

    fn interpolate(
        &self,
        at: DateTime,
        prev: &DataPoint<Self::Type>,
        next: &DataPoint<Self::Type>,
    ) -> Self::Type {
        algo::linear(at, prev, next)
    }
}

pub mod algo {
    use super::*;

    pub fn last_seen<T>(_: DateTime, prev: &DataPoint<T>, _: &DataPoint<T>) -> T
    where
        T: Clone,
    {
        prev.value.clone()
    }

    //linear interpolation or last seen
    pub fn linear<T>(at: DateTime, prev: &DataPoint<T>, next: &DataPoint<T>) -> T
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

        let interpolated_value = prev_value
            + (next_value - prev_value) * (at_time - prev_time) / (next_time - prev_time);

        interpolated_value.into()
    }
}
