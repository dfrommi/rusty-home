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
    pub fn new(data_points: impl IntoIterator<Item = DataPoint<T>>) -> Result<Self> {
        let mut values: BTreeMap<DateTime<Utc>, DataPoint<T>> = BTreeMap::new();
        for dp in data_points.into_iter() {
            values.insert(dp.timestamp, dp);
        }

        ensure!(!values.is_empty(), "data points are empty");

        Ok(Self {
            values,
            _marker: PhantomData,
        })
    }

    pub fn at_or_latest_before(&self, at: chrono::DateTime<chrono::Utc>) -> Option<DataPoint<T>> {
        self.values
            .range(..=at)
            .next_back()
            .map(|(_, v)| v)
            .cloned()
    }

    pub fn iter(&self) -> impl Iterator<Item = &DataPoint<T>> {
        self.values.values()
    }

    pub fn mean(&self) -> T {
        let sum: f64 = self.values.values().map(|dp| dp.value.clone().into()).sum();
        let count = self.values.len() as f64;
        (sum / count).into()
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
        fn test_point_before() {
            let ts = test_series();

            let dp_opt =
                ts.at_or_latest_before(Utc.with_ymd_and_hms(2024, 9, 10, 16, 30, 0).unwrap());

            let dp = assert_some(dp_opt);
            assert_eq!(
                dp.timestamp,
                Utc.with_ymd_and_hms(2024, 9, 10, 16, 0, 0).unwrap()
            );
            assert_eq!(dp.value, 20.0);
        }

        #[test]
        fn test_point_exact_match() {
            let ts = test_series();
            let dt = Utc.with_ymd_and_hms(2024, 9, 10, 16, 0, 0).unwrap();

            let dp_opt = ts.at_or_latest_before(dt);

            let dp = assert_some(dp_opt);
            assert_eq!(dp.timestamp, dt);
            assert_eq!(dp.value, 20.0);
        }

        #[test]
        fn test_no_point_before() {
            let ts = test_series();
            let dp_opt =
                ts.at_or_latest_before(Utc.with_ymd_and_hms(2024, 9, 10, 12, 0, 0).unwrap());

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
        TimeSeries::new(vec![
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
        ])
        .unwrap()
    }
}
