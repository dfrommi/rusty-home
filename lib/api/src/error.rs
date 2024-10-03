pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("item not found")]
    NotFound,
    #[error("inconsistent combination of position and area")]
    LocationDataInconsistent,
    #[error("error in database query")]
    Persistence(#[from] sqlx::Error),
    #[error("deresialzation failed")]
    Deserialisation(#[from] serde_json::error::Error),
    #[error("error parsing value to float")]
    NumberFormat(#[from] std::num::ParseFloatError),
    #[error("Invaild parameter")]
    InvalidParameter,
}
