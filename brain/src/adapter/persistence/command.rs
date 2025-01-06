use std::sync::Arc;

use anyhow::Result;
use api::command::{
    db::schema::{DbCommandSource, DbCommandState},
    Command, CommandExecution, CommandSource, CommandState, CommandTarget,
};
use sqlx::PgPool;
use support::{
    t,
    time::{DateTime, DateTimeRange},
};

use crate::port::{CommandAccess, CommandStore};

impl CommandAccess for super::Database {
    #[tracing::instrument(skip_all, fields(command_target))]
    async fn get_latest_command(
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
    async fn get_all_commands_for_target(
        &self,
        target: impl Into<CommandTarget>,
        since: DateTime,
    ) -> Result<Vec<CommandExecution>> {
        let target: CommandTarget = target.into();
        tracing::Span::current().record("command_target", tracing::field::display(&target));

        self.get_commands_using_cache(&target, since).await
    }

    async fn get_all_commands(
        &self,
        from: DateTime,
        until: DateTime,
    ) -> Result<Vec<CommandExecution>> {
        //no cache, just used from dashboard
        query_all_commands(&self.pool, None, &from, &until).await
    }
}

impl CommandStore for super::Database {
    #[tracing::instrument(skip(self))]
    async fn save_command(&self, command: Command, source: CommandSource) -> Result<()> {
        let db_command = serde_json::json!(command);
        let (db_source_type, db_source_id): (DbCommandSource, String) = source.into();

        sqlx::query!(
            r#"INSERT INTO THING_COMMAND (COMMAND, CREATED, STATUS, SOURCE_TYPE, SOURCE_ID, CORRELATION_ID) VALUES ($1, $2, $3, $4, $5, $6)"#,
            db_command,
            t!(now).into_db(),
            DbCommandState::Pending as DbCommandState,
            db_source_type as DbCommandSource,
            db_source_id,
            monitoring::TraceContext::current_correlation_id(),
        )
        .execute(&self.pool)
        .await?;

        self.invalidate_command_cache(&command.into()).await;

        Ok(())
    }
}

impl super::Database {
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

#[cfg(test)]
mod get_all_commands_since {
    use super::*;
    use api::command::PowerToggle;
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
