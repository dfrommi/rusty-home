use support::time::DateTime;

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
    pub fn map_value<U>(&self, f: impl FnOnce(&T) -> U) -> DataPoint<U> {
        let value = f(&self.value);
        DataPoint {
            value,
            timestamp: self.timestamp,
        }
    }
}
