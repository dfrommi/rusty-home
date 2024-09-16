pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("error parsing value to float")]
    NumberFormat(#[from] std::num::ParseFloatError),
    #[error("Http request error")]
    HttpError(#[from] reqwest::Error),
    #[error("Error performing database query")]
    ApiError(#[from] api::Error),
}
