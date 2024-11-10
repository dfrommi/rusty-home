use api::command::{Command, CommandSource};

use crate::{adapter::persistence::CommandRepository, prelude::*};

use anyhow::Result;

mod planning;
pub mod state;

pub use planning::{do_plan, ActionResult};

pub trait Executable {
    async fn execute(self, source: CommandSource) -> Result<()>;
}

impl<C: Into<Command>> Executable for C {
    async fn execute(self, source: CommandSource) -> Result<()> {
        home_api().execute_command(&self.into(), &source).await
    }
}
