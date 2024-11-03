pub trait OptionExt<T> {
    fn to_some(self) -> Option<Self>
    where
        Self: Sized;
}

impl<T> OptionExt<T> for T {
    fn to_some(self) -> Option<Self> {
        Some(self)
    }
}
