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
