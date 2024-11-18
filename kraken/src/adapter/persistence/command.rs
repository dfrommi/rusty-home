use api::{
    command::{
        db::schema::{DbCommandSource, DbCommandState},
        Command, CommandExecution, CommandSource, CommandState,
    },
    DbEventListener,
};

use anyhow::Result;

use crate::port::{CommandRepository, NewCommandAvailableTrigger};

use super::Database;

pub struct NewCommandAvailablePgListener {
    receiver: tokio::sync::broadcast::Receiver<()>,
}

impl NewCommandAvailablePgListener {
    pub fn new(listener: &DbEventListener) -> anyhow::Result<Self> {
        let receiver = listener.new_listener(api::THING_COMMAND_ADDED_EVENT)?;
        Ok(Self { receiver })
    }
}

impl NewCommandAvailableTrigger for NewCommandAvailablePgListener {
    async fn recv(&mut self) {
        loop {
            match self.receiver.recv().await {
                Ok(_) => break,
                Err(e) => tracing::error!("Error listening for new command: {}", e),
            }
        }
    }
}

impl CommandRepository for Database {
    //TODO handle too old commands -> expect TTL with command, store in DB and return error with message
    async fn get_command_for_processing(&self) -> Result<Option<CommandExecution<Command>>> {
        let mut tx = self.db_pool.begin().await?;

        let maybe_rec = sqlx::query!(
            r#"SELECT id, command, created, status as "status: DbCommandState", error, source_type as "source_type: DbCommandSource", source_id
                from THING_COMMAND
                where status = $1
                order by created DESC
                limit 1
                for update skip locked"#,
            DbCommandState::Pending as DbCommandState,
        )
        .fetch_optional(&mut *tx)
        .await?;

        match maybe_rec {
            None => Ok(None),
            Some(rec) => {
                let id = rec.id;

                mark_other_commands_superseeded(&mut *tx, id).await?;

                let command_res: std::result::Result<Command, serde_json::Error> =
                    serde_json::from_value(rec.command);

                let result = match command_res {
                    Ok(command) => {
                        set_command_status_in_tx(
                            &mut *tx,
                            id,
                            DbCommandState::InProgress,
                            Option::None,
                        )
                        .await?;

                        let source = CommandSource::from((rec.source_type, rec.source_id));

                        Some(CommandExecution {
                            id,
                            command,
                            state: CommandState::InProgress,
                            created: rec.created.into(),
                            source,
                        })
                    }
                    Err(e) => {
                        set_command_status_in_tx(
                            &mut *tx,
                            id,
                            DbCommandState::Error,
                            Option::Some(format!("Error reading stored command: {}", e).as_str()),
                        )
                        .await?;
                        None
                    }
                };

                tx.commit().await?;
                Ok(result)
            }
        }
    }

    async fn set_command_state_success(&self, command_id: i64) -> Result<()> {
        set_command_status_in_tx(&self.db_pool, command_id, DbCommandState::Success, None).await
    }

    async fn set_command_state_error(&self, command_id: i64, error_message: &str) -> Result<()> {
        set_command_status_in_tx(
            &self.db_pool,
            command_id,
            DbCommandState::Error,
            Some(error_message),
        )
        .await
    }
}

async fn set_command_status_in_tx(
    executor: impl sqlx::Executor<'_, Database = sqlx::Postgres>,
    command_id: i64,
    status: DbCommandState,
    error_message: Option<&str>,
) -> Result<()> {
    sqlx::query!(
        r#"UPDATE THING_COMMAND SET status = $2, error = $3 WHERE id = $1"#,
        command_id,
        status as DbCommandState,
        error_message
    )
    .execute(executor)
    .await
    .map(|_| ())
    .map_err(Into::into)
}

async fn mark_other_commands_superseeded(
    executor: impl sqlx::Executor<'_, Database = sqlx::Postgres>,
    excluded_command_id: i64,
) -> Result<(), sqlx::Error> {
    sqlx::query!(r#"
        WITH excluded_command AS (
            SELECT command->'type' as type, command->'device' as device FROM THING_COMMAND WHERE id = $1
        )
        UPDATE THING_COMMAND
        SET status = $2, error = $3
        WHERE id != $1
        AND status = $4
        AND command->'type' = (SELECT type FROM excluded_command)
        AND command->'device' = (SELECT device FROM excluded_command)"#, 
        excluded_command_id,
        DbCommandState::Error as DbCommandState,
        format!("Command was superseded by {}", excluded_command_id), 
        DbCommandState::Pending as DbCommandState)
    .execute(executor)
    .await?;

    Ok(())
}
