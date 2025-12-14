use crate::core::time::DateTime;

#[derive(Debug, Clone)]
pub struct DataPoint<V> {
    pub value: V,
    pub timestamp: DateTime,
}

impl<V> DataPoint<V> {
    pub fn new(value: V, timestamp: DateTime) -> Self {
        Self { value, timestamp }
    }
}

impl<T> DataPoint<T> {
    pub fn at(self, timestamp: DateTime) -> DataPoint<T> {
        DataPoint {
            value: self.value,
            timestamp,
        }
    }

    pub fn with<U>(&self, value: U) -> DataPoint<U> {
        DataPoint {
            value,
            timestamp: self.timestamp,
        }
    }

    pub fn map_value<U>(&self, f: impl FnOnce(&T) -> U) -> DataPoint<U> {
        let value = f(&self.value);
        DataPoint {
            value,
            timestamp: self.timestamp,
        }
    }
}

impl<V: std::fmt::Display> std::fmt::Display for DataPoint<V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} @ {})", self.value, self.timestamp.to_human_readable())
    }
}

impl std::ops::Not for DataPoint<bool> {
    type Output = DataPoint<bool>;

    fn not(self) -> Self::Output {
        DataPoint {
            value: !self.value,
            timestamp: self.timestamp,
        }
    }
}

impl std::ops::BitOr for DataPoint<bool> {
    type Output = DataPoint<bool>;

    fn bitor(self, rhs: Self) -> Self::Output {
        DataPoint {
            value: self.value | rhs.value,
            timestamp: std::cmp::max(self.timestamp, rhs.timestamp),
        }
    }
}

impl std::ops::BitAnd for DataPoint<bool> {
    type Output = DataPoint<bool>;

    fn bitand(self, rhs: Self) -> Self::Output {
        DataPoint {
            value: self.value & rhs.value,
            timestamp: std::cmp::max(self.timestamp, rhs.timestamp),
        }
    }
}
