use std::collections::{BTreeMap, BTreeSet};

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

        //process sorted by timestamp to ensure deterministic behavior when interting
        let values = {
            let mut vals: Vec<DataPoint<T>> = values.into_iter().collect();
            vals.sort_by_key(|dp| dp.timestamp);
            vals
        };

        for dp in values {
            data.insert(dp.timestamp, dp);
        }

        Self { data }
    }

    pub fn by_reducing2<A, B>(
        data1: (&DataFrame<A>, impl Interpolator<A>),
        data2: (&DataFrame<B>, impl Interpolator<B>),
        f: impl Fn(&DataPoint<A>, &DataPoint<B>) -> T,
    ) -> Self
    where
        A: Clone,
        B: Clone,
    {
        let (df1, interp1) = data1;
        let (df2, interp2) = data2;

        let mut all_timestamps: BTreeSet<DateTime> = BTreeSet::new();
        all_timestamps.extend(df1.data.keys());
        all_timestamps.extend(df2.data.keys());
        all_timestamps.insert(t!(now));

        let mut new_data_points = Vec::new();

        for dt in all_timestamps {
            let value1 = interp1.interpolate_df(dt, df1);
            let value2 = interp2.interpolate_df(dt, df2);

            if let (Some(value1), Some(value2)) = (value1, value2) {
                let new_value = f(&DataPoint::new(value1, dt), &DataPoint::new(value2, dt));
                new_data_points.push(DataPoint::new(new_value, dt));
            }
        }

        DataFrame::new(new_data_points)
    }

    pub fn retain_range(
        mut self,
        range: &DateTimeRange,
        start_interpolator: impl Interpolator<T>,
        end_interpolator: impl Interpolator<T>,
    ) -> Self {
        let start = *range.start();
        let end = *range.end();

        if !self.data.contains_key(&start)
            && let Some(dp_at_start) = start_interpolator.interpolate_df(start, &self)
        {
            self.data.insert(start, DataPoint::new(dp_at_start, start));
        }

        if !self.data.contains_key(&end)
            && let Some(dp_at_end) = end_interpolator.interpolate_df(end, &self)
        {
            self.data.insert(end, DataPoint::new(dp_at_end, end));
        }

        self.data.retain(|k, _| *k >= start && *k <= end);

        self
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

    pub fn last2(&self) -> Option<(&DataPoint<T>, &DataPoint<T>)> {
        let mut iter = self.data.values().rev();
        let last = iter.next()?;
        let second_last = iter.next()?;
        Some((second_last, last))
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

impl<T: Clone> IntoIterator for DataFrame<T> {
    type Item = DataPoint<T>;
    type IntoIter = std::vec::IntoIter<DataPoint<T>>;

    fn into_iter(self) -> Self::IntoIter {
        //TODO optimize to avoid collecting into vec first
        self.data.into_values().collect::<Vec<_>>().into_iter()
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
