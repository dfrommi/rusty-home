use chrono::{DateTime, Utc};
use sqlx::PgPool;
use state::*;

use self::command::{Command, CommandExecution, CommandTarget};

pub mod command;
mod error;
pub mod state;

pub use crate::error::{Error, Result};

#[derive(Debug)]
pub struct HomeApi {
    db_pool: PgPool,
}

#[derive(Debug, Clone)]
pub struct BackendApi {
    db_pool: PgPool,
}

impl HomeApi {
    pub fn new(db_pool: PgPool) -> Self {
        Self { db_pool }
    }

    pub async fn get_latest<'a, C: ChannelId>(&self, id: &'a C) -> Result<DataPoint<C::ValueType>>
    where
        &'a C: Into<DbChannelId>,
        C::ValueType: From<f64>,
    {
        crate::state::db::get_latest(&(self.db_pool), id).await
    }

    pub async fn get_covering<'a, C: ChannelId>(
        &self,
        id: &'a C,
        start: DateTime<Utc>,
    ) -> Result<Vec<DataPoint<C::ValueType>>>
    where
        &'a C: Into<DbChannelId>,
        C::ValueType: From<f64>,
    {
        crate::state::db::get_covering(&(self.db_pool), id, start).await
    }

    pub async fn execute_command(&self, command: &Command) -> Result<()> {
        crate::command::db::add_command(&(self.db_pool), command).await
    }

    pub async fn get_latest_command(
        &self,
        target: &CommandTarget,
    ) -> Result<Option<CommandExecution>> {
        command::db::get_latest_for_target(&self.db_pool, target).await
    }
}

impl BackendApi {
    pub fn new(db_pool: PgPool) -> Self {
        Self { db_pool }
    }

    pub async fn get_command_for_processing(&self) -> Result<Option<Command>> {
        command::db::get_command_for_processing(&self.db_pool).await
    }

    pub async fn add_thing_value(
        &self,
        value: &ChannelValue,
        timestamp: &DateTime<Utc>,
    ) -> Result<()> {
        state::db::add_thing_value(&self.db_pool, value, timestamp).await
    }
}
