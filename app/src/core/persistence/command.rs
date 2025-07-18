use crate::core::time::DateTimeRange;
use crate::home::command::*;
use crate::t;
use anyhow::Result;
use schema::*;

// Command Execution & Processing
// High-level command execution logic with deduplication and state validation
impl super::Database {
    //TODO handle too old commands -> expect TTL with command, store in DB and return error with message
    pub async fn get_command_for_processing(&self) -> Result<Option<CommandExecution>> {
        let mut tx = self.pool.begin().await?;

        let maybe_rec = sqlx::query!(
            r#"SELECT id, command, created, status as "status: DbCommandState", error, source_type as "source_type: DbCommandSource", source_id, correlation_id
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

                let command_res: std::result::Result<Command, serde_json::Error> = serde_json::from_value(rec.command);

                let result = match command_res {
                    Ok(command) => {
                        set_command_status_in_tx(&mut *tx, id, DbCommandState::InProgress, Option::None).await?;

                        let source = CommandSource::from((rec.source_type, rec.source_id));

                        Some(CommandExecution {
                            id,
                            command,
                            state: CommandState::InProgress,
                            created: rec.created.into(),
                            source,
                            correlation_id: rec.correlation_id,
                        })
                    }
                    Err(e) => {
                        set_command_status_in_tx(
                            &mut *tx,
                            id,
                            DbCommandState::Error,
                            Option::Some(format!("Error reading stored command: {e}").as_str()),
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
}

// Command Persistence & State Management
// Methods for saving commands and managing their execution state
impl super::Database {
    #[tracing::instrument(skip(self))]
    pub async fn save_command(
        &self,
        command: &Command,
        source: CommandSource,
        correlation_id: Option<String>,
    ) -> Result<()> {
        let db_command = serde_json::json!(command);
        let (db_source_type, db_source_id): (DbCommandSource, String) = source.into();

        sqlx::query!(
            r#"INSERT INTO THING_COMMAND (COMMAND, CREATED, STATUS, SOURCE_TYPE, SOURCE_ID, CORRELATION_ID) VALUES ($1, $2, $3, $4, $5, $6)"#,
            db_command,
            t!(now).into_db(),
            DbCommandState::Pending as DbCommandState,
            db_source_type as DbCommandSource,
            db_source_id,
            correlation_id
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn set_command_state_success(&self, command_id: i64) -> Result<()> {
        set_command_status_in_tx(&self.pool, command_id, DbCommandState::Success, None).await
    }

    pub async fn set_command_state_error(&self, command_id: i64, error_message: &str) -> Result<()> {
        set_command_status_in_tx(&self.pool, command_id, DbCommandState::Error, Some(error_message)).await
    }
}

// Helper methods for cache management
impl super::Database {
    pub async fn query_all_commands(
        &self,
        target: Option<CommandTarget>,
        range: &DateTimeRange,
    ) -> Result<Vec<CommandExecution>> {
        let db_target = target.map(|j| serde_json::json!(j));

        let records = sqlx::query!(
            r#"SELECT id, command, created, status as "status: DbCommandState", error, source_type as "source_type: DbCommandSource", source_id, correlation_id
                from THING_COMMAND 
                where (command @> $1 or $1 is null)
                and created >= $2
                and created <= $3
                and created <= $4
                order by created asc"#,
            db_target,
            range.start().into_db(),
            range.end().into_db(),
            t!(now).into_db()
        )
        .fetch_all(&self.pool)
        .await?;

        records
            .into_iter()
            .map(|row| {
                let source = CommandSource::from((row.source_type, row.source_id));
                Ok(CommandExecution {
                    id: row.id,
                    command: serde_json::from_value(row.command)?,
                    state: CommandState::from((row.status, row.error)),
                    created: row.created.into(),
                    source,
                    correlation_id: row.correlation_id,
                })
            })
            .collect()
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
    sqlx::query!(
        r#"
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
        DbCommandState::Pending as DbCommandState
    )
    .execute(executor)
    .await?;

    Ok(())
}

pub mod schema {
    #[derive(Debug, Clone, sqlx::Type)]
    #[sqlx(type_name = "VARCHAR", rename_all = "snake_case")]
    pub enum DbCommandState {
        Pending,
        InProgress,
        Success,
        Error,
    }

    #[derive(Debug, Clone, PartialEq, Eq, sqlx::Type)]
    #[sqlx(type_name = "VARCHAR", rename_all = "snake_case")]
    pub enum DbCommandSource {
        System,
        User,
    }
}

pub mod mapper {
    use super::*;
    use crate::home::command::{CommandSource, CommandState};

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

    impl From<(DbCommandSource, String)> for CommandSource {
        fn from(value: (DbCommandSource, String)) -> Self {
            match value.0 {
                DbCommandSource::System => CommandSource::System(value.1),
                DbCommandSource::User => CommandSource::User(value.1),
            }
        }
    }

    impl From<CommandSource> for (DbCommandSource, String) {
        fn from(val: CommandSource) -> Self {
            match val {
                CommandSource::System(id) => (DbCommandSource::System, id.to_owned()),
                CommandSource::User(id) => (DbCommandSource::User, id.to_owned()),
            }
        }
    }
}

#[cfg(test)]
mod get_all_commands_since {
    use super::super::Database;
    use super::*;
    use crate::home::command::PowerToggle;
    use crate::t;
    use sqlx::PgPool;

    #[sqlx::test(migrations = "../migrations")]
    async fn test_command_found(db_pool: PgPool) {
        //GIVEN
        let db = Database::new(db_pool);

        for (power_on, timestampe) in [
            (true, t!(4 minutes ago)),
            (false, t!(6 minutes ago)),
            (true, t!(10 minutes ago)),
        ] {
            insert_command(
                &db,
                &Command::SetPower {
                    device: PowerToggle::Dehumidifier,
                    power_on,
                },
                timestampe,
            )
            .await;
        }

        insert_command(
            &db,
            &Command::SetPower {
                device: PowerToggle::LivingRoomNotificationLight,
                power_on: true,
            },
            t!(2 minutes ago),
        )
        .await;

        //WHEN
        let result = db
            .query_all_commands(
                Some(PowerToggle::Dehumidifier.into()),
                &DateTimeRange::new(t!(8 minutes ago), t!(now)),
            )
            .await
            .unwrap();

        //THEN
        assert_eq!(result.len(), 2);
        assert_eq!(
            result[0].command,
            Command::SetPower {
                device: PowerToggle::Dehumidifier,
                power_on: false,
            }
        );
        assert_eq!(
            result[1].command,
            Command::SetPower {
                device: PowerToggle::Dehumidifier,
                power_on: true,
            }
        );
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn test_command_without_target_filter(db_pool: PgPool) {
        //GIVEN
        let db = Database::new(db_pool);

        insert_command(
            &db,
            &Command::SetPower {
                device: PowerToggle::LivingRoomNotificationLight,
                power_on: true,
            },
            t!(2 minutes ago),
        )
        .await;

        insert_command(
            &db,
            &Command::SetPower {
                device: PowerToggle::Dehumidifier,
                power_on: true,
            },
            t!(4 minutes ago),
        )
        .await;

        //WHEN
        let result = db
            .query_all_commands(None, &DateTimeRange::new(t!(1 hours ago), t!(now)))
            .await
            .unwrap();

        //THEN
        assert_eq!(result.len(), 2);
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn test_no_command(db_pool: PgPool) {
        //GIVEN
        let db = Database::new(db_pool);

        insert_command(
            &db,
            &Command::SetPower {
                device: PowerToggle::Dehumidifier,
                power_on: true,
            },
            t!(10 minutes ago),
        )
        .await;

        //WHEN
        let result = db
            .query_all_commands(
                Some(PowerToggle::Dehumidifier.into()),
                &DateTimeRange::new(t!(8 minutes ago), t!(now)),
            )
            .await
            .unwrap();

        //THEN
        assert_eq!(result.len(), 0);
    }

    async fn insert_command(db: &Database, command: &Command, at: DateTime) {
        sqlx::query!(
            r#"INSERT INTO THING_COMMAND (COMMAND, CREATED, STATUS, SOURCE_TYPE, SOURCE_ID) VALUES ($1, $2, $3, $4, $5)"#,
            serde_json::to_value(command).unwrap(),
            at.into_db(),
            DbCommandState::Pending as DbCommandState,
            DbCommandSource::System as DbCommandSource,
            "unit-test".to_owned()
        )
        .execute(&db.pool)
        .await
        .unwrap();
    }
}
