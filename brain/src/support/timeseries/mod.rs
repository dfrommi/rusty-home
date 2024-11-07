pub mod interpolate;

use chrono::{DateTime, Duration, Utc};
use interpolate::Interpolatable;
use std::collections::BTreeMap;

use crate::adapter::persistence::DataPoint;
use anyhow::{ensure, Result};

pub struct TimeSeries<T: Clone + Interpolatable> {
    values: BTreeMap<DateTime<Utc>, T>,
}

impl<T: Clone + Interpolatable> TimeSeries<T> {
    pub fn new(
        data_points: impl IntoIterator<Item = DataPoint<T>>,
        start_at: DateTime<Utc>,
    ) -> Result<Self> {
        let mut values: BTreeMap<DateTime<Utc>, T> = BTreeMap::new();
        for dp in data_points.into_iter() {
            values.insert(dp.timestamp, dp.value);
        }

        ensure!(!values.is_empty(), "data points are empty");

        if let Some(interpolated) = Self::interpolate(start_at, &values) {
            values.insert(start_at, interpolated);
        }

        Ok(Self {
            values: values.split_off(&start_at), //remove all values before start
        })
    }

    pub fn combined<U: Clone + Interpolatable, V: Clone + Interpolatable, F>(
        first_series: &TimeSeries<U>,
        second_series: &TimeSeries<V>,
        merge: F,
    ) -> Result<Self>
    where
        F: Fn(&U, &V) -> T,
    {
        let mut dps: Vec<DataPoint<T>> = Vec::new();

        for (first_timestamp, first_value) in first_series.values.iter() {
            if let Some(second_dp) = second_series.at(*first_timestamp) {
                let value = (merge)(first_value, &second_dp.value);
                let timestamp = std::cmp::max(*first_timestamp, second_dp.timestamp);
                dps.push(DataPoint { value, timestamp });
            }
        }

        for (second_timestamp, second_value) in second_series.values.iter() {
            if let Some(first_dp) = first_series.at(*second_timestamp) {
                let value = (merge)(&first_dp.value, second_value);
                let timestamp = std::cmp::max(first_dp.timestamp, *second_timestamp);
                dps.push(DataPoint { value, timestamp });
            }
        }

        let since = std::cmp::max(first_series.starting_at(), second_series.starting_at());
        Self::new(dps, since)
    }

    #[allow(dead_code)]
    pub fn first(&self) -> DataPoint<T> {
        let (timestamp, value) = self.values.first_key_value().unwrap();

        DataPoint {
            timestamp: *timestamp,
            value: value.clone(),
        }
    }

    pub fn last(&self) -> DataPoint<T> {
        let (timestamp, value) = self.values.last_key_value().unwrap();

        DataPoint {
            timestamp: *timestamp,
            value: value.clone(),
        }
    }

    //linear interpolation or last seen
    pub fn at(&self, at: chrono::DateTime<chrono::Utc>) -> Option<DataPoint<T>> {
        Self::interpolate(at, &self.values).map(|v| DataPoint {
            timestamp: at,
            value: v,
        })
    }

    pub fn min(&self) -> DataPoint<T>
    where
        for<'a> &'a T: PartialOrd,
    {
        let (timestamp, value) = self
            .values
            .iter()
            .min_by(|(_, a), (_, b)| a.partial_cmp(&b).unwrap_or(std::cmp::Ordering::Equal))
            .expect("Internal error: map should not be empty");

        DataPoint {
            timestamp: *timestamp,
            value: value.clone(),
        }
    }

    fn starting_at(&self) -> DateTime<Utc> {
        *self
            .values
            .keys()
            .next()
            .expect("Internal error: map should not be empty")
    }

    pub fn with_duration(&self) -> Vec<DataPoint<(T, Duration)>> {
        self.current_and_next()
            .into_iter()
            .map(|((timestamp, value), next)| DataPoint {
                timestamp: *timestamp,
                value: (
                    value.clone(),
                    next.map_or(Utc::now(), |n| *n.0) - *timestamp,
                ),
            })
            .collect::<Vec<_>>()
    }

    fn current_and_next(&self) -> Vec<((&DateTime<Utc>, &T), Option<(&DateTime<Utc>, &T)>)> {
        let mut result = vec![];
        let mut iter = self.values.iter().peekable();

        while let Some((current_timestamp, value)) = iter.next() {
            let next: Option<(&DateTime<Utc>, &T)> = iter.peek().map(|(t, v)| (*t, *v));
            result.push(((current_timestamp, value), next));
        }

        result
    }

    pub fn interpolate(
        at: chrono::DateTime<chrono::Utc>,
        values: &BTreeMap<DateTime<Utc>, T>,
    ) -> Option<T> {
        let prev = values
            .range(..=at)
            .next_back()
            .map(|(t, v)| DataPoint::new(v.clone(), *t));
        let next = values
            .range(at..)
            .next()
            .map(|(t, v)| DataPoint::new(v.clone(), *t));

        T::interpolate(at, prev.as_ref(), next.as_ref())
    }
}

//MATH FUNCTIONS
impl<T> TimeSeries<T>
where
    T: Clone + Interpolatable + From<f64>,
    for<'a> &'a T: Into<f64>,
{
    //weighted by duration
    pub fn mean(&self) -> T {
        let mut weighted_sum = 0.0;
        let mut total_duration = 0.0; //in milliseconds

        let mut iter = self.values.iter().peekable();
        while let Some((current_timestamp, current_value)) = iter.next() {
            if let Some((next_timestamp, next_value)) = iter.peek() {
                let duration = (next_timestamp.timestamp_millis()
                    - current_timestamp.timestamp_millis()) as f64;
                let current_f64 = current_value.into();
                let next_f64: f64 = (*next_value).into();

                //linear interpolated
                weighted_sum += ((current_f64 + next_f64) / 2.0) * duration;
                total_duration += duration;
            }
        }

        if total_duration == 0.0 {
            return self.values.values().next().unwrap().clone();
        }

        (weighted_sum / total_duration).into()
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

#[cfg(test)]
mod combined {
    use chrono::TimeZone;
    use support::unit::{DegreeCelsius, Percent};

    use super::*;

    #[test]
    fn single_item_per_series_out_of_range() {
        let t_series = TimeSeries::new(
            vec![DataPoint {
                timestamp: Utc.with_ymd_and_hms(2024, 11, 3, 15, 23, 46).unwrap(),
                value: DegreeCelsius(19.93),
            }],
            Utc.with_ymd_and_hms(2024, 11, 4, 5, 10, 9).unwrap(),
        )
        .unwrap();

        let h_series = TimeSeries::new(
            vec![DataPoint {
                timestamp: Utc.with_ymd_and_hms(2024, 11, 3, 15, 23, 47).unwrap(),
                value: Percent(61.1),
            }],
            Utc.with_ymd_and_hms(2024, 11, 4, 5, 10, 9).unwrap(),
        )
        .unwrap();

        let result = TimeSeries::combined(&t_series, &h_series, |a, b| DegreeCelsius(a.0 + b.0));

        assert_eq!(result.iter().len(), 1);
    }
}
