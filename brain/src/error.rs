pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("item not found")]
    NotFound,
    #[error("Error performing database query")]
    ApiError(#[from] api::Error),
    #[error("Error in timeseries processing")]
    TimeSeriesError(#[from] polars::error::PolarsError),
    #[error("error in database query")]
    Persistence(#[from] sqlx::Error),
}
