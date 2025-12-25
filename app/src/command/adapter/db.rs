use crate::{
    command::{Command, CommandExecution, CommandState, CommandTarget},
    core::{id::ExternalId, time::DateTimeRange},
    t,
    trigger::UserTriggerId,
};
use anyhow::Result;
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
        correlation_id: Option<String>,
    ) -> Result<CommandExecution> {
        self.insert_command(command, source, user_trigger_id, correlation_id, DbCommandState::InProgress)
            .await
    }

    async fn insert_command(
        &self,
        command: &Command,
        source: &ExternalId,
        user_trigger_id: Option<UserTriggerId>,
        correlation_id: Option<String>,
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
            correlation_id,
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

    pub async fn query_all_commands(
        &self,
        target: Option<CommandTarget>,
        range: &DateTimeRange,
    ) -> Result<Vec<CommandExecution>> {
        let db_target = target.map(|j| serde_json::json!(j));

        let records = sqlx::query!(
            r#"(SELECT id as "id!", command as "command!", created as "created", status as "status!: DbCommandState", error, source_type as "source_type!", source_id as "source_id!", correlation_id, user_trigger_id
                from thing_command 
                where (command @> $1 or $1 is null)
                and created >= $2
                and created <= $3)
            UNION ALL
            (SELECT id, command, created, status, error, source_type, source_id, correlation_id, user_trigger_id
                from thing_command 
                where (command @> $1 or $1 is null)
                and created < $2
                order by created DESC
                limit 1)
            UNION ALL
            (SELECT id, command, created, status, error, source_type, source_id, correlation_id, user_trigger_id
                from thing_command 
                where (command @> $1 or $1 is null)
                and created > $3
                order by created ASC
                limit 1)
            order by created asc"#,
            db_target,
            range.start().into_db(),
            range.end().into_db()
        )
        .fetch_all(&self.pool)
        .await?;

        let commands = records
            .into_iter()
            .filter_map(|row| {
                let source = ExternalId::new(row.source_type, row.source_id);
                match serde_json::from_value::<Command>(row.command) {
                    Ok(command) => Some(CommandExecution {
                        id: row.id,
                        command,
                        state: CommandState::from((row.status, row.error)),
                        created: row.created.unwrap().into(),
                        source,
                        user_trigger_id: row.user_trigger_id.map(UserTriggerId::from),
                        correlation_id: row.correlation_id,
                    }),
                    Err(e) => {
                        tracing::warn!("Error mapping command with id {} from database, ignoring: {}", row.id, e);
                        None
                    }
                }
            })
            .collect();

        Ok(commands)
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

impl From<(DbCommandState, Option<String>)> for CommandState {
    fn from((status, error): (DbCommandState, Option<String>)) -> Self {
        match status {
            DbCommandState::Pending => CommandState::Pending,
            DbCommandState::InProgress => CommandState::InProgress,
            DbCommandState::Success => CommandState::Success,
            DbCommandState::Error => CommandState::Error(error.unwrap_or("unknown error".to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command::PowerToggle;

    #[sqlx::test(migrations = "../migrations")]
    async fn test_command_found(db_pool: PgPool) {
        let repo = CommandRepository::new(db_pool);

        for (power_on, timestamp) in [
            (true, t!(4 minutes ago)),
            (false, t!(6 minutes ago)),
            (true, t!(8 minutes ago)),
            (true, t!(24 minutes ago)),
            (false, t!(26 minutes ago)),
        ] {
            let cmd = Command::SetPower {
                device: PowerToggle::LivingRoomNotificationLight,
                power_on,
            };
            let source = ExternalId::new("test", "source");
            let user_trigger_id = None;
            sqlx::query!(
                r#"INSERT INTO THING_COMMAND (COMMAND, CREATED, STATUS, SOURCE_TYPE, SOURCE_ID, USER_TRIGGER_ID) VALUES ($1, $2, $3, $4, $5, $6)"#,
                serde_json::json!(cmd),
                timestamp.into_db(),
                DbCommandState::Pending as DbCommandState,
                source.type_name(),
                source.variant_name(),
                user_trigger_id as Option<UserTriggerId>
            )
            .execute(&repo.pool)
            .await
            .unwrap();
        }

        let range = DateTimeRange::new(t!(10 minutes ago), t!(now));
        let res = repo.query_all_commands(
            Some(CommandTarget::SetPower {
                device: PowerToggle::LivingRoomNotificationLight,
            }),
            &range,
        );

        assert_eq!(res.await.unwrap().len(), 5);
    }
}
