#[allow(dead_code)]
pub trait ResultExt<T> {
    fn unwrap_or_warn(self, default: T, error_message: &str) -> T;
}

impl<T> ResultExt<T> for anyhow::Result<T> {
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
