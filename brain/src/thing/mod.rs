use api::command::{Command, CommandSource};

use crate::prelude::*;

use anyhow::Result;

mod planning;
pub mod state;

pub use planning::do_plan;

pub trait Executable {
    async fn execute(self) -> Result<()>;
    async fn execute_on_behalf_of_user(self) -> Result<()>;
}

impl<C: Into<Command>> Executable for C {
    async fn execute(self) -> Result<()> {
        home_api()
            .execute_command(&self.into(), &CommandSource::System)
            .await
    }

    async fn execute_on_behalf_of_user(self) -> Result<()> {
        home_api()
            .execute_command(&self.into(), &CommandSource::User)
            .await
    }
}
