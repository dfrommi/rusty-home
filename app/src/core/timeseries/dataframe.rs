use std::collections::BTreeMap;

use crate::{
    core::{
        time::{DateTime, DateTimeRange, Duration},
        timeseries::interpolate::Interpolator,
    },
    t,
};

use super::DataPoint;

#[derive(Debug, Clone)]
pub struct DataFrame<T: Clone> {
    data: BTreeMap<DateTime, DataPoint<T>>,
}

impl<T: Clone> DataFrame<T> {
    pub fn empty() -> Self {
        Self { data: BTreeMap::new() }
    }

    pub fn new(values: impl IntoIterator<Item = DataPoint<T>>) -> Self {
        let mut data: BTreeMap<DateTime, DataPoint<T>> = BTreeMap::new();
        for dp in values {
            data.insert(dp.timestamp, dp);
        }

        Self { data }
    }

    pub fn retain_range(
        &mut self,
        range: &DateTimeRange,
        start_interpolator: impl Interpolator<T>,
        end_interpolator: impl Interpolator<T>,
    ) {
        let start = *range.start();
        let end = *range.end();

        if !self.data.contains_key(&start)
            && let Some(dp_at_start) = start_interpolator.interpolate_df(start, self)
        {
            self.data.insert(start, DataPoint::new(dp_at_start, start));
        }

        if !self.data.contains_key(&end)
            && let Some(dp_at_end) = end_interpolator.interpolate_df(end, self)
        {
            self.data.insert(end, DataPoint::new(dp_at_end, end));
        }

        self.data.retain(|k, _| *k >= start && *k <= end);
    }

    pub fn retain_range_with_context_before(&self, range: &DateTimeRange) -> Self {
        let in_range = self.data.range(*range.start()..=*range.end());
        let before = self.prev(*range.start());

        let mut points = Vec::new();

        if let Some(dp) = before {
            points.push(dp.clone());
        }

        for (_, dp) in in_range {
            points.push(dp.clone());
        }

        DataFrame::new(points)
    }

    pub fn map<U: Clone>(&self, f: impl Fn(&DataPoint<T>) -> U) -> DataFrame<U> {
        let values = self.data.values().map(|dp| {
            let ts = dp.timestamp;
            DataPoint::new(f(dp), ts)
        });

        DataFrame::new(values)
    }

    pub fn map_interval<U, F>(&self, f: F) -> Vec<U>
    where
        F: Fn(&DataPoint<T>, &DataPoint<T>) -> U,
    {
        self.current_and_next()
            .into_iter()
            .filter_map(|(current, next)| next.map(|n| f(current, n)))
            .collect::<Vec<_>>()
    }

    pub fn latest_where(&self, predicate: impl Fn(&DataPoint<T>) -> bool) -> Option<&DataPoint<T>> {
        self.data.values().rev().find(|dp| predicate(dp))
    }

    pub fn insert(&mut self, dp: DataPoint<T>)
    where
        T: PartialEq,
    {
        //depuplicate values
        let prev = self.prev(dp.timestamp);
        match prev {
            Some(prev_dp) if prev_dp.value == dp.value => return,
            _ => self.data.insert(dp.timestamp, dp),
        };
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    pub fn non_empty(self) -> Option<Self> {
        if self.is_empty() { None } else { Some(self) }
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn last(&self) -> Option<&DataPoint<T>> {
        self.data.values().next_back()
    }

    pub fn prev_or_at(&self, at: DateTime) -> Option<&DataPoint<T>> {
        self.data.range(..=at).next_back().map(|(_, v)| v)
    }

    pub fn prev(&self, at: DateTime) -> Option<&DataPoint<T>> {
        self.data.range(..at).next_back().map(|(_, v)| v)
    }

    pub fn next(&self, at: DateTime) -> Option<&DataPoint<T>> {
        self.data.range(at..).next().map(|(_, v)| v)
    }

    pub fn with_duration_until_next_dp(&self) -> Vec<DataPoint<(T, Duration)>> {
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
}

impl<T, U> From<&DataFrame<T>> for DataFrame<U>
where
    T: Clone + Into<U>,
    U: Clone,
{
    fn from(val: &DataFrame<T>) -> Self {
        val.map(|dp| dp.value.clone().into())
    }
}
