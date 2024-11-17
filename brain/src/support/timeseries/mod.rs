pub mod interpolate;

use interpolate::Interpolatable;
use std::collections::BTreeMap;
use support::{
    t,
    time::{DateTime, Duration},
    DataPoint,
};

use anyhow::{ensure, Result};

pub struct TimeSeries<T: Clone + Interpolatable> {
    values: BTreeMap<DateTime, T>,
}

impl<T: Clone + Interpolatable> TimeSeries<T> {
    pub fn new(
        data_points: impl IntoIterator<Item = DataPoint<T>>,
        start_at: DateTime,
    ) -> Result<Self> {
        let mut values: BTreeMap<DateTime, T> = BTreeMap::new();
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

    pub fn len(&self) -> usize {
        self.values.len()
    }

    //linear interpolation or last seen
    pub fn at(&self, at: DateTime) -> Option<DataPoint<T>> {
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
            .min_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .expect("Internal error: map should not be empty");

        DataPoint {
            timestamp: *timestamp,
            value: value.clone(),
        }
    }

    fn starting_at(&self) -> DateTime {
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
                    next.map_or(t!(now), |n| *n.0).elapsed_since(*timestamp),
                ),
            })
            .collect::<Vec<_>>()
    }

    fn current_and_next(&self) -> Vec<((&DateTime, &T), Option<(&DateTime, &T)>)> {
        let mut result = vec![];
        let mut iter = self.values.iter().peekable();

        while let Some((current_timestamp, value)) = iter.next() {
            let next: Option<(&DateTime, &T)> = iter.peek().map(|(t, v)| (*t, *v));
            result.push(((current_timestamp, value), next));
        }

        result
    }

    pub fn interpolate(at: DateTime, values: &BTreeMap<DateTime, T>) -> Option<T> {
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
                let duration: f64 = next_timestamp
                    .elapsed_since(*current_timestamp)
                    .as_secs_f64();
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

            let dp_opt = ts.at(DateTime::from_iso("2024-09-10T16:30:00Z").unwrap());

            let dp = assert_some(dp_opt);
            assert_eq!(
                dp.timestamp,
                DateTime::from_iso("2024-09-10T16:30:00Z").unwrap()
            );
            assert_eq!(dp.value.0, 22.5);
        }

        #[test]
        fn test_point_exact_match() {
            let ts = test_series();
            let dt = DateTime::from_iso("2024-09-10T16:00:00Z").unwrap();

            let dp_opt = ts.at(dt);

            let dp = assert_some(dp_opt);
            assert_eq!(dp.timestamp, dt);
            assert_eq!(dp.value.0, 20.0);
        }

        #[test]
        fn test_no_point_before() {
            let ts = test_series();
            let dp_opt = ts.at(DateTime::from_iso("2024-09-10T12:00:00Z").unwrap());

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
                    timestamp: DateTime::from_iso("2024-09-10T14:00:00Z").unwrap(),
                    value: DegreeCelsius(10.0),
                },
                DataPoint {
                    timestamp: DateTime::from_iso("2024-09-10T18:00:00Z").unwrap(),
                    value: DegreeCelsius(30.0),
                },
                DataPoint {
                    timestamp: DateTime::from_iso("2024-09-10T16:00:00Z").unwrap(),
                    value: DegreeCelsius(20.0),
                },
            ],
            DateTime::from_iso("2024-09-10T13:00:00Z").unwrap(),
        )
        .unwrap()
    }
}

#[cfg(test)]
mod combined {
    use support::unit::{DegreeCelsius, Percent};

    use super::*;

    #[test]
    fn single_item_per_series_out_of_range() {
        let t_series = TimeSeries::new(
            vec![DataPoint {
                timestamp: DateTime::from_iso("2024-11-03T15:23:46Z").unwrap(),
                value: DegreeCelsius(19.93),
            }],
            DateTime::from_iso("2024-11-04T05:10:09Z").unwrap(),
        )
        .unwrap();

        let h_series = TimeSeries::new(
            vec![DataPoint {
                timestamp: DateTime::from_iso("2024-11-03T15:23:47Z").unwrap(),
                value: Percent(61.1),
            }],
            DateTime::from_iso("2024-11-04T05:10:09Z").unwrap(),
        )
        .unwrap();

        let result = TimeSeries::combined(&t_series, &h_series, |a, b| DegreeCelsius(a.0 + b.0));

        assert_eq!(result.iter().len(), 1);
    }
}
