use anyhow::Result;
use api::command::{
    db::schema::{DbCommandSource, DbCommandState},
    Command, CommandExecution, CommandSource, CommandState, CommandTarget,
};
use sqlx::PgPool;
use support::{t, time::DateTime};

use crate::port::{CommandAccess, CommandStore};

impl<DB> CommandAccess for DB
where
    DB: AsRef<PgPool>,
{
    #[tracing::instrument(skip_all, fields(command_target))]
    async fn get_latest_command(
        &self,
        target: impl Into<CommandTarget>,
        since: DateTime,
    ) -> Result<Option<CommandExecution>> {
        let target: CommandTarget = target.into();
        tracing::Span::current().record("command_target", tracing::field::display(&target));

        let mut all_commands =
            get_all_commands(self.as_ref(), Some(target), since, t!(now)).await?;
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

        get_all_commands(self.as_ref(), Some(target), since, t!(now)).await
    }

    async fn get_all_commands(
        &self,
        from: DateTime,
        until: DateTime,
    ) -> Result<Vec<CommandExecution>> {
        get_all_commands(self.as_ref(), None, from, until).await
    }
}

impl<DB> CommandStore for DB
where
    DB: AsRef<PgPool>,
{
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
        .execute(self.as_ref())
        .await?;

        Ok(())
    }
}

async fn get_all_commands(
    db_pool: &PgPool,
    target: Option<CommandTarget>,
    from: DateTime,
    until: DateTime,
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
        let result = get_all_commands(
            &db_pool,
            Some(PowerToggle::Dehumidifier.into()),
            t!(8 minutes ago),
            t!(now),
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
        let result = get_all_commands(&db_pool, None, t!(1 hours ago), t!(now))
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
        let result = get_all_commands(
            &db_pool,
            Some(PowerToggle::Dehumidifier.into()),
            t!(8 minutes ago),
            t!(now),
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
