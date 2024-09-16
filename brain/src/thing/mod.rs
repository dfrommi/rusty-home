use api::command::Command;

use crate::prelude::*;

pub mod state;

pub trait Executable {
    async fn execute(&self) -> Result<()>;
}

impl Executable for Command {
    async fn execute(&self) -> Result<()> {
        Ok(home_api().execute_command(self).await?)
    }
}
