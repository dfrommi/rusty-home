pub mod dataframe;
pub mod datapoint;
pub mod interpolate;

use crate::core::time::{DateTime, DateTimeRange, Duration};
pub use dataframe::DataFrame;
pub use datapoint::DataPoint;
use interpolate::Estimatable;

use anyhow::{Result, bail};

pub struct TimeSeries<T: Estimatable> {
    context: T,
    values: DataFrame<T::ValueType>,
    num_estimated: usize,
}

impl<C: Estimatable> TimeSeries<C> {
    pub fn new(context: C, df: &DataFrame<C::ValueType>, range: DateTimeRange) -> Result<Self> {
        //not using retain_range as it could lead to empty dataframe before interpolation
        let mut dps_in_range = df
            .iter()
            .filter(|dp| range.contains(&dp.timestamp))
            .cloned()
            .collect::<Vec<_>>();

        let mut num_estimated = 0;

        let start_at = *range.start();
        if let Some(interpolated) = Self::interpolate_or_guess(&context, start_at, df) {
            dps_in_range.push(DataPoint::new(interpolated, start_at));
            num_estimated += 1;
        }

        let end_at = *range.end();
        if let Some(interpolated) = Self::interpolate_or_guess(&context, end_at, df) {
            dps_in_range.push(DataPoint::new(interpolated, end_at));
            num_estimated += 1;
        }

        Ok(Self {
            context,
            values: DataFrame::new(dps_in_range)?,
            num_estimated,
        })
    }

    pub fn combined<U, V, F>(
        first_series: &TimeSeries<U>,
        second_series: &TimeSeries<V>,
        context: C,
        merge: F,
    ) -> Result<Self>
    where
        F: Fn(&U::ValueType, &V::ValueType) -> C::ValueType,
        U: Estimatable,
        V: Estimatable,
    {
        let df = DataFrame::<C::ValueType>::combined(first_series, second_series, merge)?;
        let range = first_series.range().intersection_with(&second_series.range());
        Self::new(context, &df, range)
    }

    pub fn reduce<F>(context: C, all_series: Vec<TimeSeries<C>>, reduce: F) -> Result<TimeSeries<C>>
    where
        F: Fn(&C::ValueType, &C::ValueType) -> C::ValueType,
        C: Clone,
    {
        if all_series.is_empty() {
            bail!("No series to reduce");
        }

        let mut all_series = all_series;
        let mut merged = all_series.remove(0);

        for ts in all_series {
            merged = TimeSeries::combined(&merged, &ts, context.clone(), |a, b| reduce(a, b))?
        }

        Ok(merged)
    }

    pub fn map<T: Estimatable, F>(self, context: T, f: F) -> TimeSeries<T>
    where
        F: Fn(&DataPoint<C::ValueType>) -> T::ValueType,
        C: Clone,
    {
        TimeSeries::new(context, &self.values.map(f), self.range())
            .expect("Internal error: Error creating data frame from non-empty datapoints")
    }

    pub fn context(&self) -> C
    where
        C: Clone,
    {
        self.context.clone()
    }

    //linear interpolation or last seen
    pub fn at(&self, at: DateTime) -> Option<DataPoint<C::ValueType>> {
        Self::interpolate_or_guess(&self.context, at, &self.values).map(|v| DataPoint {
            timestamp: at,
            value: v,
        })
    }

    fn interpolate_or_guess(context: &C, at: DateTime, data: &DataFrame<C::ValueType>) -> Option<C::ValueType> {
        context
            .interpolate(at, data)
            //reconsider implicit last-seen. Needed to avoid empty time-series in some cases
            .or_else(|| interpolate::algo::last_seen(at, data))
    }
}

//DataFrame delegates
impl<T: Estimatable> TimeSeries<T> {
    pub fn inner(&self) -> &DataFrame<T::ValueType> {
        &self.values
    }

    pub fn len_non_estimated(&self) -> usize {
        self.values.len() - self.num_estimated
    }

    pub fn range(&self) -> DateTimeRange {
        self.values.range()
    }

    pub fn first(&self) -> &DataPoint<T::ValueType> {
        self.values.first()
    }

    pub fn last(&self) -> &DataPoint<T::ValueType> {
        self.values.last()
    }

    pub fn with_duration_until_next_dp(&self) -> Vec<DataPoint<(T::ValueType, Duration)>> {
        self.values.with_duration_until_next_dp()
    }
}

//MATH FUNCTIONS
//TODO handle area in a saver way: use general duration, not just seconds. Also make result
//interpolatable. Maybe introduce an area type.
impl<T: Estimatable> TimeSeries<T>
where
    T::ValueType: From<f64>,
    for<'a> &'a T::ValueType: Into<f64>,
{
    pub fn mean(&self) -> T::ValueType {
        let (weighted_sum, total_duration) = self.weighted_sum_and_duration_in_type_hours();

        if total_duration == 0.0 {
            weighted_sum.into()
        } else {
            (weighted_sum / total_duration).into()
        }
    }

    pub fn area_in_type_hours(&self) -> f64 {
        self.weighted_sum_and_duration_in_type_hours().0
    }

    //weighted by duration
    fn weighted_sum_and_duration_in_type_hours(&self) -> (f64, f64) {
        let mut weighted_sum = 0.0;
        let mut total_duration_h = 0.0;

        let area_series = self.area_series_in_unit_hours();

        for dp in area_series.iter() {
            let (value, duration) = (&dp.value.0, &dp.value.1);
            weighted_sum += value;
            total_duration_h += duration.as_hours_f64();
        }

        if total_duration_h == 0.0 {
            weighted_sum = (&self.values.first().value).into();
        }

        (weighted_sum, total_duration_h)
    }

    pub fn area_series_in_unit_hours(&self) -> DataFrame<(f64, Duration)> {
        let mut datapoints: Vec<DataPoint<(f64, Duration)>> = vec![DataPoint::new(
            (0.0, Duration::millis(0)),
            self.values.first().timestamp,
        )];

        let mut iter = self.values.iter().map(|v| v.timestamp).peekable();
        while let Some(current_timestamp) = iter.next() {
            if let Some(next_timestamp) = iter.peek() {
                let ref_value: T::ValueType = self
                    .at(DateTime::midpoint(&current_timestamp, next_timestamp))
                    .expect("Unexpected error. Could not get value in the middle of two existing values")
                    .value;

                let duration = next_timestamp.elapsed_since(current_timestamp);
                let duration_h: f64 = duration.as_hours_f64();

                //good enough approximation for mean in range. Correct for linear and last-seen interpolation
                let midpoint_f64: f64 = (&ref_value).into();

                datapoints.push(DataPoint {
                    value: (midpoint_f64 * duration_h, duration),
                    timestamp: *next_timestamp,
                });
            }
        }

        DataFrame::new(datapoints).expect("Internal error: error creating DataFrame from non-empty datapoints")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::unit::DegreeCelsius;
    use crate::home::state::Temperature;

    #[test]
    fn test_mean() {
        let ts = test_series();

        //mean value per time range, multiplied by duration, last section uses last seen value
        let expected = ((10.0 + 20.0) / 2.0 * 2.0 + (20.0 + 30.0) / 2.0 * 2.0 + (30.0 + 30.0) / 2.0 * 1.0) / 5.0;

        assert_eq!(ts.mean().0, expected);
    }

    #[test]
    fn test_area_in_type_hours() {
        let ts = test_series();
        let expected = (10.0 + 20.0) / 2.0 * 2.0 + (20.0 + 30.0) / 2.0 * 2.0 + (30.0 + 30.0) / 2.0 * 1.0;
        assert_eq!(ts.area_in_type_hours(), expected);
    }

    mod at {
        use super::*;

        #[test]
        fn test_points_around() {
            let ts = test_series();

            let dp_opt = ts.at(DateTime::from_iso("2024-09-10T16:30:00Z").unwrap());

            let dp = assert_some(dp_opt);
            assert_eq!(dp.timestamp, DateTime::from_iso("2024-09-10T16:30:00Z").unwrap());
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
            &DataFrame::new(vec![
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
            ])
            .unwrap(),
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
    use crate::core::unit::{DegreeCelsius, Percent};
    use crate::home::state::DewPoint;
    use crate::home::state::{RelativeHumidity, Temperature};

    use super::*;

    #[test]
    fn single_item_per_series_out_of_range() {
        let t_series = TimeSeries::new(
            Temperature::LivingRoom,
            &DataFrame::new(vec![DataPoint {
                timestamp: DateTime::from_iso("2024-11-03T15:23:46Z").unwrap(),
                value: DegreeCelsius(19.93),
            }])
            .unwrap(),
            DateTimeRange::since(DateTime::from_iso("2024-11-04T05:10:09Z").unwrap()),
        )
        .unwrap();

        let h_series = TimeSeries::new(
            RelativeHumidity::LivingRoom,
            &DataFrame::new(vec![DataPoint {
                timestamp: DateTime::from_iso("2024-11-03T15:23:47Z").unwrap(),
                value: Percent(61.1),
            }])
            .unwrap(),
            DateTimeRange::since(DateTime::from_iso("2024-11-04T05:10:09Z").unwrap()),
        )
        .unwrap();

        let result = TimeSeries::combined(&t_series, &h_series, DewPoint::LivingRoom, |a, b| DegreeCelsius(a.0 + b.0));

        assert_eq!(result.iter().len(), 1);
    }
}
