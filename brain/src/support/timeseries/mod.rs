pub mod interpolate;

use interpolate::Estimatable;
use std::collections::BTreeMap;
use support::{
    t,
    time::{DateTime, DateTimeRange, Duration},
    DataPoint,
};

use anyhow::{ensure, Result};

pub struct TimeSeries<T: Estimatable> {
    context: T,
    values: BTreeMap<DateTime, T::Type>,
    range: DateTimeRange,
}

impl<T: Estimatable> TimeSeries<T> {
    pub fn new(
        context: T,
        data_points: impl IntoIterator<Item = DataPoint<T::Type>>,
        range: DateTimeRange,
    ) -> Result<Self> {
        let mut values: BTreeMap<DateTime, T::Type> = BTreeMap::new();
        for dp in data_points.into_iter() {
            values.insert(dp.timestamp, dp.value);
        }

        ensure!(!values.is_empty(), "data points are empty");

        let start_at = range.start();
        if let Some(interpolated) = Self::interpolate_or_guess(&context, start_at, &values) {
            values.insert(start_at, interpolated);
        }

        let end_at = range.end();
        if let Some(interpolated) = Self::interpolate_or_guess(&context, end_at, &values) {
            values.insert(end_at, interpolated);
        }

        Ok(Self {
            context,
            values,
            range,
        })
    }

    pub fn combined<U, V, F>(
        first_series: &TimeSeries<U>,
        second_series: &TimeSeries<V>,
        context: T,
        merge: F,
    ) -> Result<Self>
    where
        F: Fn(&U::Type, &V::Type) -> T::Type,
        U: Estimatable,
        V: Estimatable,
    {
        let mut dps: Vec<DataPoint<T::Type>> = Vec::new();

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

        let range = DateTimeRange::new(
            std::cmp::max(first_series.range.start(), second_series.range.start()),
            std::cmp::min(first_series.range.end(), second_series.range.end()),
        );

        Self::new(context, dps, range)
    }

    pub fn len(&self) -> usize {
        self.values.len()
    }

    //linear interpolation or last seen
    pub fn at(&self, at: DateTime) -> Option<DataPoint<T::Type>> {
        Self::interpolate_or_guess(&self.context, at, &self.values).map(|v| DataPoint {
            timestamp: at,
            value: v,
        })
    }

    pub fn min(&self) -> DataPoint<T::Type>
    where
        for<'a> &'a T::Type: PartialOrd,
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

    pub fn with_duration(&self) -> Vec<DataPoint<(T::Type, Duration)>> {
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

    fn current_and_next(&self) -> Vec<((&DateTime, &T::Type), Option<(&DateTime, &T::Type)>)> {
        let mut result = vec![];
        let mut iter = self.values.iter().peekable();

        while let Some((current_timestamp, value)) = iter.next() {
            let next: Option<(&DateTime, &T::Type)> = iter.peek().map(|(t, v)| (*t, *v));
            result.push(((current_timestamp, value), next));
        }

        result
    }

    fn interpolate_or_guess(
        context: &T,
        at: DateTime,
        values: &BTreeMap<DateTime, T::Type>,
    ) -> Option<T::Type> {
        let prev = values
            .range(..=at)
            .next_back()
            .map(|(t, v)| DataPoint::new(v.clone(), *t));
        let next = values
            .range(at..)
            .next()
            .map(|(t, v)| DataPoint::new(v.clone(), *t));

        //TODO handle prediction (linear interpolation)
        match (prev, next) {
            (Some(prev), Some(next)) => {
                let value = context.interpolate(at, &prev, &next);
                Some(value)
            }
            (Some(prev), None) => Some(prev.value.clone()),
            _ => None,
        }
    }
}

//MATH FUNCTIONS
impl<T: Estimatable> TimeSeries<T>
where
    T::Type: From<f64>,
    for<'a> &'a T::Type: Into<f64>,
{
    pub fn mean(&self) -> T::Type {
        let (weighted_sum, total_duration) = self.weighted_sum_and_duration_in_type_secs();

        if total_duration == 0.0 {
            weighted_sum.into()
        } else {
            (weighted_sum / total_duration).into()
        }
    }

    pub fn area_in_type_hours(&self) -> f64 {
        let (weighted_sum_secs, _) = self.weighted_sum_and_duration_in_type_secs();
        weighted_sum_secs / 3600.0
    }

    //weighted by duration
    fn weighted_sum_and_duration_in_type_secs(&self) -> (f64, f64) {
        let mut weighted_sum = 0.0;
        let mut total_duration = 0.0; //in milliseconds

        let mut iter = self.values.keys().peekable();
        while let Some(current_timestamp) = iter.next() {
            if let Some(next_timestamp) = iter.peek() {
                let ref_value: T::Type = self
                    .at(DateTime::midpoint(current_timestamp, next_timestamp))
                    .expect("Unexpected error. Could not get value in the middle of two existing values")
                    .value;

                let duration: f64 = next_timestamp
                    .elapsed_since(*current_timestamp)
                    .as_secs_f64();

                //good enough approximation for mean in range. Correct for linear and last-seen interpolation
                let midpoint_f64: f64 = (&ref_value).into();

                weighted_sum += midpoint_f64 * duration;
                total_duration += duration;
            }
        }

        if total_duration == 0.0 {
            weighted_sum = self.values.values().next().unwrap().into();
        }

        (weighted_sum, total_duration)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use api::state::Temperature;
    use support::unit::DegreeCelsius;

    #[test]
    fn test_mean() {
        let ts = test_series();

        //mean value per time range, multiplied by duration, last section uses last seen value
        let expected =
            ((10.0 + 20.0) / 2.0 * 2.0 + (20.0 + 30.0) / 2.0 * 2.0 + (30.0 + 30.0) / 2.0 * 1.0)
                / 5.0;

        assert_eq!(ts.mean().0, expected);
    }

    #[test]
    fn test_area_in_type_hours() {
        let ts = test_series();
        let expected =
            (10.0 + 20.0) / 2.0 * 2.0 + (20.0 + 30.0) / 2.0 * 2.0 + (30.0 + 30.0) / 2.0 * 1.0;
        assert_eq!(ts.area_in_type_hours(), expected);
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

    fn test_series() -> TimeSeries<Temperature> {
        TimeSeries::new(
            Temperature::Outside,
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
            DateTimeRange::new(
                DateTime::from_iso("2024-09-10T13:00:00Z").unwrap(),
                DateTime::from_iso("2024-09-10T19:00:00Z").unwrap(),
            ),
        )
        .unwrap()
    }
}

#[cfg(test)]
mod combined {
    use crate::thing::state::DewPoint;
    use api::state::{RelativeHumidity, Temperature};
    use support::unit::{DegreeCelsius, Percent};

    use super::*;

    #[test]
    fn single_item_per_series_out_of_range() {
        let t_series = TimeSeries::new(
            Temperature::LivingRoomDoor,
            vec![DataPoint {
                timestamp: DateTime::from_iso("2024-11-03T15:23:46Z").unwrap(),
                value: DegreeCelsius(19.93),
            }],
            DateTimeRange::since(DateTime::from_iso("2024-11-04T05:10:09Z").unwrap()),
        )
        .unwrap();

        let h_series = TimeSeries::new(
            RelativeHumidity::LivingRoomDoor,
            vec![DataPoint {
                timestamp: DateTime::from_iso("2024-11-03T15:23:47Z").unwrap(),
                value: Percent(61.1),
            }],
            DateTimeRange::since(DateTime::from_iso("2024-11-04T05:10:09Z").unwrap()),
        )
        .unwrap();

        let result =
            TimeSeries::combined(&t_series, &h_series, DewPoint::LivingRoomDoor, |a, b| {
                DegreeCelsius(a.0 + b.0)
            });

        assert_eq!(result.iter().len(), 1);
    }
}
