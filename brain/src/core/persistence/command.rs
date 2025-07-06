use std::sync::Arc;

use crate::{home::command::*, port::CommandExecutionResult};
use anyhow::Result;
use infrastructure::TraceContext;
use schema::*;
use sqlx::PgPool;
use support::{
    t,
    time::{DateTime, DateTimeRange},
};

impl super::Database {
    pub async fn execute(
        &self,
        command: Command,
        source: crate::home::command::CommandSource,
    ) -> anyhow::Result<CommandExecutionResult> {
        let target: CommandTarget = command.clone().into();
        let last_execution = self
            .get_latest_command(target, t!(48 hours ago))
            .await?
            .filter(|e| e.source == source && e.command == command)
            .map(|e| e.created);

        //wait until roundtrip is completed. State might not have been updated yet
        let was_just_executed = last_execution.map_or(false, |dt| dt > t!(30 seconds ago));

        if was_just_executed {
            return Ok(CommandExecutionResult::Skipped);
        }

        let was_latest_execution = last_execution.is_some();
        let is_reflected_in_state = command.is_reflected_in_state(self).await?;

        if !was_latest_execution || !is_reflected_in_state {
            self.save_command(command, source, TraceContext::current_correlation_id())
                .await?;
            Ok(CommandExecutionResult::Triggered)
        } else {
            Ok(CommandExecutionResult::Skipped)
        }
    }

    #[tracing::instrument(skip_all, fields(command_target))]
    pub async fn get_latest_command(
        &self,
        target: impl Into<CommandTarget>,
        since: DateTime,
    ) -> Result<Option<CommandExecution>> {
        let target: CommandTarget = target.into();
        tracing::Span::current().record("command_target", tracing::field::display(&target));

        //This is inefficient and modifies and copies data too often. Needs to be optimized
        let mut all_commands = self.get_commands_using_cache(&target, since).await?;
        Ok(all_commands.pop())
    }

    #[tracing::instrument(skip_all, fields(command_target))]
    pub async fn get_all_commands_for_target(
        &self,
        target: impl Into<CommandTarget>,
        since: DateTime,
    ) -> Result<Vec<CommandExecution>> {
        let target: CommandTarget = target.into();
        tracing::Span::current().record("command_target", tracing::field::display(&target));

        self.get_commands_using_cache(&target, since).await
    }

    pub async fn get_all_commands(
        &self,
        from: DateTime,
        until: DateTime,
    ) -> Result<Vec<CommandExecution>> {
        //no cache, just used from dashboard
        query_all_commands(&self.pool, None, &from, &until).await
    }

    #[tracing::instrument(skip(self))]
    async fn save_command(
        &self,
        command: Command,
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

        self.invalidate_command_cache(&command.into()).await;

        Ok(())
    }

    fn cmd_caching_range(&self) -> DateTimeRange {
        let now = t!(now);
        DateTimeRange::new(now - self.cmd_cache_duration.clone(), now)
    }

    pub async fn invalidate_command_cache(&self, target: &CommandTarget) {
        tracing::debug!("Invalidating command cache for target {:?}", target);
        self.cmd_cache.invalidate(target).await;
    }

    pub async fn get_commands_using_cache(
        &self,
        target: &CommandTarget,
        since: DateTime,
    ) -> Result<Vec<CommandExecution>> {
        let cached = self
            .cmd_cache
            .try_get_with(target.clone(), async {
                tracing::debug!("No command-cache entry found for target {:?}", target);
                let range = self.cmd_caching_range();

                query_all_commands(&self.pool, Some(target.clone()), range.start(), range.end())
                    .await
                    .map(|cmds| Arc::new((range.start().clone(), cmds)))
            })
            .await
            .map_err(|e| {
                anyhow::anyhow!(
                    "Error initializing command cache for target {:?}: {:?}",
                    target,
                    e
                )
            })?;

        if since < cached.0 {
            tracing::info!(
                ?since,
                offset = %since.elapsed().to_iso_string(),
                cache_start = %cached.0,
                "Requested time range is before cached commands, querying database"
            );
            return query_all_commands(&self.pool, Some(target.clone()), &since, &t!(now)).await;
        }

        let commands: Vec<CommandExecution> = cached
            .1
            .iter()
            .filter(|&cmd| cmd.created >= since)
            .cloned()
            .collect();

        Ok(commands)
    }

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
                            correlation_id: rec.correlation_id,
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

    pub async fn set_command_state_success(&self, command_id: i64) -> Result<()> {
        set_command_status_in_tx(&self.pool, command_id, DbCommandState::Success, None).await
    }

    pub async fn set_command_state_error(
        &self,
        command_id: i64,
        error_message: &str,
    ) -> Result<()> {
        set_command_status_in_tx(
            &self.pool,
            command_id,
            DbCommandState::Error,
            Some(error_message),
        )
        .await
    }
}

async fn query_all_commands(
    db_pool: &PgPool,
    target: Option<CommandTarget>,
    from: &DateTime,
    until: &DateTime,
) -> Result<Vec<CommandExecution>> {
    let db_target = target.map(|j| serde_json::json!(j));

    let records = sqlx::query!(
            r#"SELECT id, command, created, status as "status: DbCommandState", error, source_type as "source_type: DbCommandSource", source_id, correlation_id
                from THING_COMMAND 
                where (command @> $1 or $1 is null)
                and created >= $2
                and created <= $3
                order by created asc"#,
            db_target,
            from.into_db(),
            until.into_db(),
        )
        .fetch_all(db_pool)
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

    #[derive(Debug, Clone, sqlx::FromRow)]
    pub struct DbThingCommandRow {
        pub id: i64,
        pub command: serde_json::Value,
        pub timestamp: chrono::DateTime<chrono::Utc>,
        pub status: DbCommandState,
        pub error: Option<String>,
        pub source: DbCommandSource,
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
                DbCommandState::Error => {
                    CommandState::Error(error.unwrap_or("unknown error".to_string()))
                }
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
    use super::*;
    use crate::home::command::PowerToggle;
    use sqlx::PgPool;
    use support::t;

    #[sqlx::test(migrations = "../migrations")]
    async fn test_command_found(db_pool: PgPool) {
        //GIVEN
        for (power_on, timestampe) in [
            (true, t!(4 minutes ago)),
            (false, t!(6 minutes ago)),
            (true, t!(10 minutes ago)),
        ] {
            insert_command(
                &db_pool,
                &Command::SetPower {
                    device: PowerToggle::Dehumidifier,
                    power_on,
                },
                timestampe,
            )
            .await;
        }

        insert_command(
            &db_pool,
            &Command::SetPower {
                device: PowerToggle::LivingRoomNotificationLight,
                power_on: true,
            },
            t!(2 minutes ago),
        )
        .await;

        //WHEN
        let result = query_all_commands(
            &db_pool,
            Some(PowerToggle::Dehumidifier.into()),
            &t!(8 minutes ago),
            &t!(now),
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
        insert_command(
            &db_pool,
            &Command::SetPower {
                device: PowerToggle::LivingRoomNotificationLight,
                power_on: true,
            },
            t!(2 minutes ago),
        )
        .await;

        insert_command(
            &db_pool,
            &Command::SetPower {
                device: PowerToggle::Dehumidifier,
                power_on: true,
            },
            t!(4 minutes ago),
        )
        .await;

        //WHEN
        let result = query_all_commands(&db_pool, None, &t!(1 hours ago), &t!(now))
            .await
            .unwrap();

        //THEN
        assert_eq!(result.len(), 2);
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn test_no_command(db_pool: PgPool) {
        //GIVEN
        insert_command(
            &db_pool,
            &Command::SetPower {
                device: PowerToggle::Dehumidifier,
                power_on: true,
            },
            t!(10 minutes ago),
        )
        .await;

        //WHEN
        let result = query_all_commands(
            &db_pool,
            Some(PowerToggle::Dehumidifier.into()),
            &t!(8 minutes ago),
            &t!(now),
        )
        .await
        .unwrap();

        //THEN
        assert_eq!(result.len(), 0);
    }

    async fn insert_command(db_pool: &PgPool, command: &Command, at: DateTime) {
        sqlx::query!(
            r#"INSERT INTO THING_COMMAND (COMMAND, CREATED, STATUS, SOURCE_TYPE, SOURCE_ID) VALUES ($1, $2, $3, $4, $5)"#,
            serde_json::to_value(command).unwrap(),
            at.into_db(),
            DbCommandState::Pending as DbCommandState,
            DbCommandSource::System as DbCommandSource,
            "unit-test".to_owned()
        )
        .execute(db_pool)
        .await
        .unwrap();
    }
}
