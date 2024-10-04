use goap::{Effects, Preconditions};

use anyhow::Result;

use super::HomeState;

pub mod dehumidify;

pub trait Action: Preconditions<HomeState> + Effects<HomeState> {
    async fn start(&self) -> Result<()>;
    async fn stop(&self) -> Result<()>;
    async fn is_running(&self) -> bool;
    async fn is_enabled(&self) -> bool;
}
