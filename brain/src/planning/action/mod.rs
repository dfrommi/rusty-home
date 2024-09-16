use goap::{Effects, Preconditions};

use crate::error::Result;

use super::HomeState;

pub mod dehumidify;

pub trait Action: Preconditions<HomeState> + Effects<HomeState> {
    fn start(&self) -> Result<()>;
    fn stop(&self) -> Result<()>;
    fn is_running(&self) -> bool;
    fn is_enabled(&self) -> bool;
}
