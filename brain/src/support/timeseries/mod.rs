use chrono::{DateTime, Utc};
use std::{collections::BTreeMap, fmt::Debug};

use crate::adapter::persistence::DataPoint;
use anyhow::{ensure, Result};

#[derive(Debug)]
pub struct TimeSeries<T> {
    values: BTreeMap<DateTime<Utc>, DataPoint<T>>,
}

//TODO less clone, better usage of references, maybe not always via f64
impl<T> TimeSeries<T>
where
    T: From<f64> + Clone + Debug,
    for<'a> &'a T: Into<f64>,
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
        })
    }

    pub fn combined<U, V, F>(
        first_series: &TimeSeries<U>,
        second_series: &TimeSeries<V>,
        merge: F,
    ) -> Result<Self>
    where
        U: From<f64> + Clone + Debug,
        for<'a> &'a U: Into<f64>,
        V: From<f64> + Clone + Debug,
        for<'b> &'b V: Into<f64>,
        F: Fn(&U, &V) -> T,
    {
        let mut dps: Vec<DataPoint<T>> = Vec::new();

        for first_dp in first_series.iter() {
            if let Some(second_dp) = second_series.at(first_dp.timestamp) {
                let value = (merge)(&first_dp.value, &second_dp.value);
                let timestamp = std::cmp::max(first_dp.timestamp, second_dp.timestamp);
                dps.push(DataPoint { value, timestamp });
            }
        }

        for second_dp in second_series.iter() {
            if let Some(first_dp) = first_series.at(second_dp.timestamp) {
                let value = (merge)(&first_dp.value, &second_dp.value);
                let timestamp = std::cmp::max(first_dp.timestamp, second_dp.timestamp);
                dps.push(DataPoint { value, timestamp });
            }
        }

        let since = std::cmp::max(first_series.starting_at(), second_series.starting_at());
        Self::new(dps, since)
    }

    #[allow(dead_code)]
    pub fn first(&self) -> DataPoint<T> {
        self.values
            .values()
            .next()
            .expect("Internal error: map should not be empty")
            .clone()
    }

    pub fn last(&self) -> DataPoint<T> {
        self.values
            .values()
            .last()
            .expect("Internal error: map should not be empty")
            .clone()
    }

    //linear interpolation or last seen
    pub fn at(&self, at: chrono::DateTime<chrono::Utc>) -> Option<DataPoint<T>> {
        interpolate(&self.values, at)
    }

    pub fn min(&self) -> DataPoint<T> {
        self.values
            .values()
            .min_by(|&dp_a, &dp_b| {
                let a: f64 = (&dp_a.value).into();
                let b: f64 = (&dp_b.value).into();
                a.partial_cmp(&b).unwrap_or(std::cmp::Ordering::Equal)
            })
            .expect("Internal error: map should not be empty")
            .clone()
    }

    fn starting_at(&self) -> DateTime<Utc> {
        *self
            .values
            .keys()
            .next()
            .expect("Internal error: map should not be empty")
    }

    fn iter(&self) -> impl Iterator<Item = &DataPoint<T>> {
        self.values.values()
    }

    pub fn mean(&self) -> T {
        let mut weighted_sum = 0.0;
        let mut total_duration = 0.0;

        let mut iter = self.values.values().peekable();
        while let Some(current) = iter.next() {
            if let Some(next) = iter.peek() {
                let duration = (next.timestamp - current.timestamp).num_seconds() as f64;
                let current_f64 = (&current.value).into();
                let next_f64 = (&next.value).into();

                //linear interpolated
                weighted_sum += ((current_f64 + next_f64) / 2.0) * duration;
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
    T: From<f64> + Clone,
    for<'a> &'a T: Into<f64>,
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

            let prev_value: f64 = (&prev_dp.value).into();
            let next_value: f64 = (&next_dp.value).into();

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
    use support::unit::DegreeCelsius;

    #[test]
    fn test_mean() {
        let ts = test_series();
        assert_eq!(ts.mean().0, 20.0);
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
            assert_eq!(dp.value.0, 22.5);
        }

        #[test]
        fn test_point_exact_match() {
            let ts = test_series();
            let dt = Utc.with_ymd_and_hms(2024, 9, 10, 16, 0, 0).unwrap();

            let dp_opt = ts.at(dt);

            let dp = assert_some(dp_opt);
            assert_eq!(dp.timestamp, dt);
            assert_eq!(dp.value.0, 20.0);
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

        let dps: Vec<&DataPoint<DegreeCelsius>> = ts.iter().collect();

        assert_eq!(
            dps[0].timestamp,
            Utc.with_ymd_and_hms(2024, 9, 10, 14, 0, 0).unwrap()
        );
        assert_eq!(dps[0].value.0, 10.0);

        assert_eq!(
            dps[1].timestamp,
            Utc.with_ymd_and_hms(2024, 9, 10, 16, 0, 0).unwrap()
        );
        assert_eq!(dps[1].value.0, 20.0);

        assert_eq!(
            dps[2].timestamp,
            Utc.with_ymd_and_hms(2024, 9, 10, 18, 0, 0).unwrap()
        );
        assert_eq!(dps[2].value.0, 30.0);
    }

    fn assert_some<T>(val: Option<T>) -> T {
        assert!(val.is_some());
        val.unwrap()
    }

    fn test_series() -> TimeSeries<DegreeCelsius> {
        TimeSeries::new(
            vec![
                DataPoint {
                    timestamp: Utc.with_ymd_and_hms(2024, 9, 10, 14, 0, 0).unwrap(),
                    value: DegreeCelsius(10.0),
                },
                DataPoint {
                    timestamp: Utc.with_ymd_and_hms(2024, 9, 10, 18, 0, 0).unwrap(),
                    value: DegreeCelsius(30.0),
                },
                DataPoint {
                    timestamp: Utc.with_ymd_and_hms(2024, 9, 10, 16, 0, 0).unwrap(),
                    value: DegreeCelsius(20.0),
                },
            ],
            Utc.with_ymd_and_hms(2024, 9, 10, 13, 0, 0).unwrap(),
        )
        .unwrap()
    }
}
