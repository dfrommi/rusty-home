use crate::{
    core::{time::Duration, timeseries::DataPoint},
    t,
};

pub struct RateOfChange<T> {
    delta: T,
    duration: Duration,
}

impl<T: Clone> RateOfChange<T> {
    pub fn new(delta: T, duration: Duration) -> Self {
        RateOfChange { delta, duration }
    }

    pub fn from_dps(dp1: &DataPoint<T>, dp2: &DataPoint<T>) -> Self
    where
        T: std::ops::Sub<Output = T>,
    {
        let (prev, next) = if dp1.timestamp <= dp2.timestamp {
            (dp1, dp2)
        } else {
            (dp2, dp1)
        };

        let delta = next.value.clone() - prev.value.clone();
        let duration = next.timestamp.elapsed_since(prev.timestamp);

        RateOfChange { delta, duration }
    }

    pub fn per_hour(&self) -> T
    where
        T: std::ops::Mul<f64, Output = T>,
    {
        self.per(t!(1 hours))
    }

    pub fn per_minute(&self) -> T
    where
        T: std::ops::Mul<f64, Output = T>,
    {
        self.per(t!(1 minutes))
    }

    pub fn per(&self, duration: Duration) -> T
    where
        T: std::ops::Mul<f64, Output = T>,
    {
        if self.duration < t!(1 seconds) {
            return self.delta.clone() * 0.0;
        }

        let factor = duration.as_secs_f64() / self.duration.as_secs_f64();
        self.delta.clone() * factor
    }
}

impl<T> PartialEq for RateOfChange<T>
where
    T: std::ops::Mul<f64, Output = T> + PartialEq + Clone,
{
    fn eq(&self, other: &Self) -> bool {
        self.per_minute() == other.per_minute()
    }
}

// impl<T> Eq for RateOfChange<T> where T: std::ops::Mul<f64, Output = T> + PartialEq + Clone {}

impl<T> PartialOrd for RateOfChange<T>
where
    T: std::ops::Mul<f64, Output = T> + PartialOrd + Clone,
{
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.per_minute().partial_cmp(&other.per_minute())
    }
}
