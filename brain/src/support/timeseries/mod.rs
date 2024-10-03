use polars::frame::DataFrame;
use polars::prelude::*;
use std::{fmt::Debug, marker::PhantomData};

use crate::{adapter::persistence::DataPoint, prelude::*};

const TIME: &str = "time";
const VALUE: &str = "value";

#[derive(Debug)]
pub struct TimeSeries<T> {
    df: DataFrame,
    _marker: PhantomData<T>,
}

impl<T> TimeSeries<T>
where
    T: From<f64> + Into<f64> + Debug,
{
    pub fn new(data_points: impl IntoIterator<Item = DataPoint<T>>) -> Result<Self> {
        let mut timestamps: Vec<chrono::NaiveDateTime> = Vec::new();
        let mut values: Vec<f64> = Vec::new();

        for dp in data_points {
            timestamps.push(dp.timestamp.naive_utc());
            values.push(dp.value.into());
        }

        if timestamps.is_empty() {
            return Err(Error::NotFound);
        }

        let df = df![
            TIME => timestamps,
            VALUE => values
        ]?
        .sort([TIME], SortMultipleOptions::default())?;

        Ok(Self {
            df,
            _marker: PhantomData,
        })
    }

    pub fn at_or_latest_before(&self, at: chrono::DateTime<chrono::Utc>) -> Option<DataPoint<T>> {
        let r = self.at_or_latest_before_int(at);
        match r {
            Ok(v) => Some(v),
            Err(e) => None,
        }
    }

    fn at_or_latest_before_int(&self, at: chrono::DateTime<chrono::Utc>) -> Result<DataPoint<T>> {
        let row = self
            .df
            .clone()
            .lazy()
            .filter(col(TIME).lt_eq(lit(at.naive_utc())))
            .tail(1)
            .collect()?;

        if row.is_empty() {
            return Err(Error::NotFound);
        }

        Ok(Self::to_datapoints(&row)?.remove(0))
    }

    pub fn iter(&self) -> impl Iterator<Item = DataPoint<T>> {
        Self::to_datapoints(&self.df).unwrap().into_iter()
    }

    pub fn mean(&self) -> T {
        self.df
            .column(VALUE)
            .expect("Internal error: value column not found")
            .mean()
            .expect("Internal error: mean can't be calculated, but dataframe should not be empty")
            .into()
    }

    fn to_datapoints(df: &DataFrame) -> Result<Vec<DataPoint<T>>> {
        let timestamps = df.column(TIME)?.datetime()?;
        let values = df.column(VALUE)?.f64()?;

        let mut dps: Vec<DataPoint<T>> = vec![];

        for i in 0..df.height() {
            let timestamp_i64 = timestamps.get(i).unwrap();
            let timestamp = chrono::DateTime::from_timestamp(
                timestamp_i64 / 1000,
                ((timestamp_i64 % 1000) * 1_000_000) as u32,
            )
            .unwrap();

            let value: f64 = values.get(i).unwrap_or(f64::NAN);

            dps.push(DataPoint {
                timestamp,
                value: value.into(),
            });
        }

        Ok(dps)
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

    #[test]
    fn test_at__point_before() {
        let ts = test_series();

        let dp_opt = ts.at_or_latest_before(Utc.with_ymd_and_hms(2024, 9, 10, 16, 30, 0).unwrap());

        let dp = assert_some(dp_opt);
        assert_eq!(
            dp.timestamp,
            Utc.with_ymd_and_hms(2024, 9, 10, 16, 0, 0).unwrap()
        );
        assert_eq!(dp.value, 20.0);
    }

    #[test]
    fn test_at__point_exact_match() {
        let ts = test_series();
        let dt = Utc.with_ymd_and_hms(2024, 9, 10, 16, 0, 0).unwrap();

        let dp_opt = ts.at_or_latest_before(dt);

        let dp = assert_some(dp_opt);
        assert_eq!(dp.timestamp, dt);
        assert_eq!(dp.value, 20.0);
    }

    #[test]
    fn test_at__no_point_before() {
        let ts = test_series();
        let dp_opt = ts.at_or_latest_before(Utc.with_ymd_and_hms(2024, 9, 10, 12, 0, 0).unwrap());

        assert!(dp_opt.is_none());
    }

    #[test]
    fn test_iter() {
        let ts = test_series();

        let dps: Vec<DataPoint<f64>> = ts.iter().collect();

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
