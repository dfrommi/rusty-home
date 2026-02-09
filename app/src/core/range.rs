#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Range<T> {
    from: T,
    to: T,
}

impl<T: PartialOrd> Range<T> {
    pub fn new(min: T, max: T) -> Self {
        if min > max {
            return Self { from: max, to: min };
        }

        Self { from: min, to: max }
    }

    pub fn from(&self) -> &T {
        &self.from
    }

    pub fn to(&self) -> &T {
        &self.to
    }

    pub fn contains(&self, value: &T) -> bool {
        value >= &self.from && value <= &self.to
    }
}

impl<T: PartialEq + PartialOrd> PartialEq for Range<T> {
    fn eq(&self, other: &Self) -> bool {
        self.from == other.from && self.to == other.to
    }
}

impl<T> std::fmt::Display for Range<T>
where
    T: std::fmt::Display + PartialOrd,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} - {}", self.from, self.to)
    }
}

impl<T> From<Range<T>> for (T, T) {
    fn from(val: Range<T>) -> Self {
        (val.from, val.to)
    }
}
