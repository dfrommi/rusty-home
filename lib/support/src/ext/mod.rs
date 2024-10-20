pub trait ToSome {
    fn to_some(self) -> Option<Self>
    where
        Self: Sized;
}

impl<T> ToSome for T {
    fn to_some(self) -> Option<Self> {
        Some(self)
    }
}

pub trait ToOk {
    fn to_ok<E>(self) -> Result<Self, E>
    where
        Self: Sized;
}

impl<T> ToOk for T {
    fn to_ok<E>(self) -> Result<Self, E> {
        Ok(self)
    }
}
