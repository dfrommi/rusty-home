pub trait ResultExt<T> {
    fn unwrap_or_warn(self, default: T, error_message: &str) -> T;
}

impl<T> ResultExt<T> for anyhow::Result<T> {
    //fn unwrap_or_warn(self, default: T, error_message: &str) -> T {
    fn unwrap_or_warn(self, default: T, error_message: &str) -> T {
        match self {
            Ok(value) => value,
            Err(e) => {
                tracing::warn!("{}: {:?}", error_message, e);
                default
            }
        }
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
