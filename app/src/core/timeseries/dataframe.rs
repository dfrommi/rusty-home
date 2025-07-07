use std::collections::BTreeMap;

use crate::{
    core::time::{DateTime, DateTimeRange, Duration},
    t,
};
use anyhow::ensure;

use super::DataPoint;

#[derive(Debug, Clone)]
pub struct DataFrame<T> {
    data: BTreeMap<DateTime, DataPoint<T>>,
}

impl<T> DataFrame<T> {
    pub fn new(values: impl IntoIterator<Item = DataPoint<T>>) -> anyhow::Result<Self> {
        let mut data: BTreeMap<DateTime, DataPoint<T>> = BTreeMap::new();
        for dp in values {
            data.insert(dp.timestamp, dp);
        }

        ensure!(!data.is_empty(), "data frames must not be empty");

        Ok(Self { data })
    }

    pub fn retain_range(&self, range: &DateTimeRange) -> anyhow::Result<Self>
    where
        T: Clone,
    {
        Self::new(
            self.data
                .iter()
                .filter(|(k, _)| *k >= range.start() && *k <= range.end())
                .map(|(_, v)| v.clone()),
        )
    }

    pub fn map<U>(&self, f: impl Fn(&DataPoint<T>) -> U) -> DataFrame<U> {
        let values = self.data.values().map(|dp| {
            let ts = dp.timestamp;
            DataPoint::new(f(dp), ts)
        });

        DataFrame::new(values).expect("Internal error: Error creating data frame of non-empty datapoints")
    }

    pub fn insert(&mut self, dp: DataPoint<T>) {
        self.data.insert(dp.timestamp, dp);
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn range(&self) -> DateTimeRange {
        DateTimeRange::new(self.first().timestamp, self.last().timestamp)
    }

    pub fn first(&self) -> &DataPoint<T> {
        self.data.first_key_value().unwrap().1
    }

    pub fn last(&self) -> &DataPoint<T> {
        self.data.last_key_value().unwrap().1
    }

    pub fn prev_or_at(&self, at: DateTime) -> Option<&DataPoint<T>> {
        self.data.range(..=at).next_back().map(|(_, v)| v)
    }

    pub fn next(&self, at: DateTime) -> Option<&DataPoint<T>> {
        self.data.range(at..).next().map(|(_, v)| v)
    }

    pub fn min(&self) -> &DataPoint<T>
    where
        T: PartialOrd,
    {
        self.data
            .iter()
            .min_by(|(_, a), (_, b)| a.value.partial_cmp(&b.value).unwrap_or(std::cmp::Ordering::Equal))
            .expect("Internal error: map should not be empty")
            .1
    }

    pub fn with_duration_until_next_dp(&self) -> Vec<DataPoint<(T, Duration)>>
    where
        T: Clone,
    {
        self.current_and_next()
            .into_iter()
            .map(|(current, next)| {
                current.map_value(|_| {
                    (
                        current.value.clone(),
                        next.map_or(t!(now), |n| n.timestamp).elapsed_since(current.timestamp),
                    )
                })
            })
            .collect::<Vec<_>>()
    }

    pub fn current_and_next(&self) -> Vec<(&DataPoint<T>, Option<&DataPoint<T>>)> {
        let mut result = vec![];
        let mut iter = self.data.iter().peekable();

        while let Some((_, value)) = iter.next() {
            let next = iter.peek().map(|(_, v)| *v);
            result.push((value, next));
        }

        result
    }

    pub fn iter(&self) -> impl Iterator<Item = &DataPoint<T>> {
        self.data.values()
    }
}

impl<T, U> From<&DataFrame<T>> for DataFrame<U>
where
    T: Clone + Into<U>,
{
    fn from(val: &DataFrame<T>) -> Self {
        val.map(|dp| dp.value.clone().into())
    }
}
