use api::command::{Command, CommandSource};

use crate::prelude::*;

pub mod state;

pub trait Executable {
    async fn execute(&self) -> Result<()>;
    async fn execute_on_behalf_of_user(&self) -> Result<()>;
}

impl Executable for Command {
    async fn execute(&self) -> Result<()> {
        Ok(home_api()
            .execute_command(self, &CommandSource::System)
            .await?)
    }

    async fn execute_on_behalf_of_user(&self) -> Result<()> {
        Ok(home_api()
            .execute_command(self, &CommandSource::User)
            .await?)
    }
}
