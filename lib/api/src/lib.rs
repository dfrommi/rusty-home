pub mod command;
mod error;
pub mod state;

pub use crate::error::{Error, Result};

pub use state::db::get_tag_id;
