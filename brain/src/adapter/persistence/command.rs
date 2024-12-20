use anyhow::Result;
use api::command::{
    db::schema::{DbCommandSource, DbCommandState},
    Command, CommandExecution, CommandSource, CommandState, CommandTarget,
};
use serde::de::DeserializeOwned;
use sqlx::PgPool;
use support::{t, time::DateTime};

use crate::port::{CommandAccess, CommandExecutor};

impl<DB: AsRef<PgPool>, C: Into<Command> + DeserializeOwned> CommandAccess<C> for DB {
    async fn get_latest_command(
        &self,
        target: impl Into<CommandTarget>,
        since: DateTime,
    ) -> Result<Option<CommandExecution<C>>> {
        let mut all_commands = get_all_commands::<C>(self.as_ref(), target.into(), since).await?;
        Ok(all_commands.pop())
    }

    async fn get_all_commands(
        &self,
        target: impl Into<CommandTarget>,
        since: DateTime,
    ) -> Result<Vec<CommandExecution<C>>> {
        get_all_commands::<C>(self.as_ref(), target.into(), since).await
    }

    async fn get_latest_command_source(
        &self,
        target: impl Into<CommandTarget>,
        since: DateTime,
    ) -> Result<Option<CommandSource>> {
        let maybe_command = CommandAccess::<C>::get_latest_command(self, target, since).await?;
        Ok(maybe_command.map(|c| c.source))
    }
}

impl<DB, C> CommandExecutor<C> for DB
where
    C: Into<Command>,
    DB: AsRef<PgPool>,
{
    async fn execute(&self, command: C, source: CommandSource) -> Result<()> {
        let command: Command = command.into();

        let db_command = serde_json::json!(command);
        let (db_source_type, db_source_id): (DbCommandSource, String) = source.into();

        sqlx::query!(
            r#"INSERT INTO THING_COMMAND (COMMAND, CREATED, STATUS, SOURCE_TYPE, SOURCE_ID) VALUES ($1, $2, $3, $4, $5)"#,
            db_command,
            t!(now).into_db(),
            DbCommandState::Pending as DbCommandState,
            db_source_type as DbCommandSource,
            db_source_id
        )
        .execute(self.as_ref())
        .await?;

        Ok(())
    }
}

async fn get_all_commands<C: Into<Command> + DeserializeOwned>(
    db_pool: &PgPool,
    target: CommandTarget,
    since: DateTime,
) -> Result<Vec<CommandExecution<C>>> {
    let db_target = serde_json::json!(target);

    let records = sqlx::query!(
            r#"SELECT id, command, created, status as "status: DbCommandState", error, source_type as "source_type: DbCommandSource", source_id
                from THING_COMMAND 
                where command @> $1 
                and created >= $2
                and created <= $3
                order by created asc"#,
            db_target,
            since.into_db(),
            t!(now).into_db(), //For timeshift in tests
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
            })
        })
        .collect()
}

#[cfg(test)]
mod get_all_commands_since {
    use super::*;
    use api::command::{PowerToggle, SetPower};
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
                &SetPower {
                    device: PowerToggle::Dehumidifier,
                    power_on,
                },
                timestampe,
            )
            .await;
        }

        insert_command(
            &db_pool,
            &SetPower {
                device: PowerToggle::LivingRoomNotificationLight,
                power_on: true,
            },
            t!(2 minutes ago),
        )
        .await;

        //WHEN
        let result = get_all_commands::<SetPower>(
            &db_pool,
            PowerToggle::Dehumidifier.into(),
            t!(8 minutes ago),
        )
        .await
        .unwrap();

        //THEN
        assert_eq!(result.len(), 2);
        assert_eq!(
            result[0].command,
            SetPower {
                device: PowerToggle::Dehumidifier,
                power_on: false,
            }
        );
        assert_eq!(
            result[1].command,
            SetPower {
                device: PowerToggle::Dehumidifier,
                power_on: true,
            }
        );
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn test_no_command(db_pool: PgPool) {
        //GIVEN
        insert_command(
            &db_pool,
            &SetPower {
                device: PowerToggle::Dehumidifier,
                power_on: true,
            },
            t!(10 minutes ago),
        )
        .await;

        //WHEN
        let result = get_all_commands::<SetPower>(
            &db_pool,
            PowerToggle::Dehumidifier.into(),
            t!(8 minutes ago),
        )
        .await
        .unwrap();

        //THEN
        assert_eq!(result.len(), 0);
    }

    async fn insert_command<C: Into<Command> + Clone>(db_pool: &PgPool, command: &C, at: DateTime) {
        let command: Command = command.clone().into();

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
