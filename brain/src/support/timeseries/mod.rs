mod multi_ts;

use chrono::{DateTime, Utc};
use std::{collections::BTreeMap, fmt::Debug, marker::PhantomData};

use crate::adapter::persistence::DataPoint;
use anyhow::{ensure, Result};

pub use multi_ts::MultiTimeSeriesAccess;

#[derive(Debug)]
pub struct TimeSeries<T> {
    values: BTreeMap<DateTime<Utc>, DataPoint<T>>,
    _marker: PhantomData<T>,
}

impl<T> TimeSeries<T>
where
    T: From<f64> + Into<f64> + Clone + Debug,
{
    pub fn new(
        data_points: impl IntoIterator<Item = DataPoint<T>>,
        start_at: DateTime<Utc>,
    ) -> Result<Self> {
        let mut values: BTreeMap<DateTime<Utc>, DataPoint<T>> = BTreeMap::new();
        for dp in data_points.into_iter() {
            values.insert(dp.timestamp, dp);
        }

        ensure!(!values.is_empty(), "data points are empty");

        if let Some(interpolated) = interpolate(&values, start_at) {
            values.insert(start_at, interpolated);
        }

        Ok(Self {
            values: values.split_off(&start_at), //remove all values before start
            _marker: PhantomData,
        })
    }

    //linear interpolation or last seen
    pub fn at(&self, at: chrono::DateTime<chrono::Utc>) -> Option<DataPoint<T>> {
        interpolate(&self.values, at)
    }

    pub fn iter(&self) -> impl Iterator<Item = &DataPoint<T>> {
        self.values.values()
    }

    pub fn mean(&self) -> T {
        let mut weighted_sum = 0.0;
        let mut total_duration = 0.0;

        let mut iter = self.values.values().peekable();
        while let Some(current) = iter.next() {
            if let Some(next) = iter.peek() {
                let duration = (next.timestamp - current.timestamp).num_seconds() as f64;
                //linear interpolated
                weighted_sum +=
                    ((current.value.clone().into() + next.value.clone().into()) / 2.0) * duration;
                total_duration += duration;
            }
        }

        if total_duration == 0.0 {
            return self.values.values().next().unwrap().value.clone();
        }

        (weighted_sum / total_duration).into()
    }
}

//linear interpolation or last seen
fn interpolate<T>(
    values: &BTreeMap<DateTime<Utc>, DataPoint<T>>,
    at: chrono::DateTime<chrono::Utc>,
) -> Option<DataPoint<T>>
where
    T: From<f64> + Into<f64> + Clone,
{
    if let Some(dp) = values.get(&at) {
        return Some(dp.clone());
    }

    let prev = values.range(..=at).next_back().map(|(_, dp)| dp);
    let next = values.range(at..).next().map(|(_, dp)| dp);

    match (prev, next) {
        (Some(prev_dp), Some(next_dp)) => {
            let prev_time = prev_dp.timestamp.timestamp() as f64;
            let next_time = next_dp.timestamp.timestamp() as f64;
            let at_time = at.timestamp() as f64;

            let prev_value: f64 = prev_dp.value.clone().into();
            let next_value: f64 = next_dp.value.clone().into();

            let interpolated_value = prev_value
                + (next_value - prev_value) * (at_time - prev_time) / (next_time - prev_time);

            Some(DataPoint {
                timestamp: at,
                value: interpolated_value.into(),
            })
        }

        (Some(prev_dp), None) => Some(prev_dp.clone()),

        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ::chrono::{TimeZone, Utc};

    #[test]
    fn test_mean() {
        let ts = test_series();
        assert_eq!(ts.mean(), 20.0);
    }

    mod at {
        use super::*;

        #[test]
        fn test_points_around() {
            let ts = test_series();

            let dp_opt = ts.at(Utc.with_ymd_and_hms(2024, 9, 10, 16, 30, 0).unwrap());

            let dp = assert_some(dp_opt);
            assert_eq!(
                dp.timestamp,
                Utc.with_ymd_and_hms(2024, 9, 10, 16, 30, 0).unwrap()
            );
            assert_eq!(dp.value, 22.5);
        }

        #[test]
        fn test_point_exact_match() {
            let ts = test_series();
            let dt = Utc.with_ymd_and_hms(2024, 9, 10, 16, 0, 0).unwrap();

            let dp_opt = ts.at(dt);

            let dp = assert_some(dp_opt);
            assert_eq!(dp.timestamp, dt);
            assert_eq!(dp.value, 20.0);
        }

        #[test]
        fn test_no_point_before() {
            let ts = test_series();
            let dp_opt = ts.at(Utc.with_ymd_and_hms(2024, 9, 10, 12, 0, 0).unwrap());

            assert!(dp_opt.is_none());
        }
    }

    #[test]
    fn test_iter() {
        let ts = test_series();

        let dps: Vec<&DataPoint<f64>> = ts.iter().collect();

        assert_eq!(
            dps[0].timestamp,
            Utc.with_ymd_and_hms(2024, 9, 10, 14, 0, 0).unwrap()
        );
        assert_eq!(dps[0].value, 10.0);

        assert_eq!(
            dps[1].timestamp,
            Utc.with_ymd_and_hms(2024, 9, 10, 16, 0, 0).unwrap()
        );
        assert_eq!(dps[1].value, 20.0);

        assert_eq!(
            dps[2].timestamp,
            Utc.with_ymd_and_hms(2024, 9, 10, 18, 0, 0).unwrap()
        );
        assert_eq!(dps[2].value, 30.0);
    }

    fn assert_some<T>(val: Option<T>) -> T {
        assert!(val.is_some());
        val.unwrap()
    }

    fn test_series() -> TimeSeries<f64> {
        TimeSeries::new(
            vec![
                DataPoint {
                    timestamp: Utc.with_ymd_and_hms(2024, 9, 10, 14, 0, 0).unwrap(),
                    value: 10.0,
                },
                DataPoint {
                    timestamp: Utc.with_ymd_and_hms(2024, 9, 10, 18, 0, 0).unwrap(),
                    value: 30.0,
                },
                DataPoint {
                    timestamp: Utc.with_ymd_and_hms(2024, 9, 10, 16, 0, 0).unwrap(),
                    value: 20.0,
                },
            ],
            Utc.with_ymd_and_hms(2024, 9, 10, 13, 0, 0).unwrap(),
        )
        .unwrap()
    }
}
