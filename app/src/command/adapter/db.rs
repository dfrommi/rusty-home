use crate::{
    command::{Command, CommandExecution, CommandState},
    core::id::ExternalId,
    t,
    trigger::UserTriggerId,
};
use anyhow::Result;
use infrastructure::CorrelationId;
use sqlx::PgPool;

#[derive(Debug, Clone)]
pub struct CommandRepository {
    pool: PgPool,
}

impl CommandRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn insert_command_for_processing(
        &self,
        command: &Command,
        source: &ExternalId,
        user_trigger_id: Option<UserTriggerId>,
        correlation_id: Option<CorrelationId>,
    ) -> Result<CommandExecution> {
        self.insert_command(command, source, user_trigger_id, correlation_id, DbCommandState::InProgress)
            .await
    }

    async fn insert_command(
        &self,
        command: &Command,
        source: &ExternalId,
        user_trigger_id: Option<UserTriggerId>,
        correlation_id: Option<CorrelationId>,
        state: DbCommandState,
    ) -> Result<CommandExecution> {
        let db_command = serde_json::json!(command);

        let rec = sqlx::query!(
            r#"INSERT INTO THING_COMMAND (COMMAND, CREATED, STATUS, SOURCE_TYPE, SOURCE_ID, CORRELATION_ID, USER_TRIGGER_ID) 
                VALUES ($1, $2, $3, $4, $5, $6, $7)
                RETURNING id, created"#,
            db_command,
            t!(now).into_db(),
            state as DbCommandState,
            source.type_name(),
            source.variant_name(),
            correlation_id.as_ref().map(|id| id.to_string()),
            user_trigger_id.clone() as Option<UserTriggerId>
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(CommandExecution {
            id: rec.id,
            command: command.clone(),
            state: CommandState::Pending,
            created: rec.created.into(),
            source: source.clone(),
            user_trigger_id,
            correlation_id,
        })
    }

    pub async fn set_command_state(&self, command_id: i64, state: CommandState) -> Result<()> {
        let (status, error_message) = match state {
            CommandState::Pending => (DbCommandState::Pending, None),
            CommandState::InProgress => (DbCommandState::InProgress, None),
            CommandState::Success => (DbCommandState::Success, None),
            CommandState::Error(err) => (DbCommandState::Error, Some(err)),
        };

        sqlx::query!(
            r#"UPDATE THING_COMMAND SET status = $2, error = $3 WHERE id = $1"#,
            command_id,
            status as DbCommandState,
            error_message
        )
        .execute(&self.pool)
        .await
        .map(|_| ())
        .map_err(Into::into)
    }
}

#[derive(Debug, Clone, sqlx::Type)]
#[sqlx(type_name = "VARCHAR", rename_all = "snake_case")]
pub enum DbCommandState {
    Pending,
    InProgress,
    Success,
    Error,
}
