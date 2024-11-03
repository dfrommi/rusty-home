use anyhow::Result;
use api::{
    command::{
        db::schema::{DbCommandSource, DbCommandState},
        Command, CommandExecution, CommandId, CommandSource, CommandState, CommandTarget,
    },
    get_tag_id,
    state::{db::DbValue, Channel, ChannelTypeInfo},
    EventListener,
};
use chrono::{DateTime, Utc};
use sqlx::{postgres::PgListener, PgPool};
use tokio::sync::broadcast::Receiver;

#[derive(Debug, Clone)]
pub struct DataPoint<V> {
    pub value: V,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug)]
pub struct HomeApi {
    db_pool: PgPool,
}

#[derive(Debug)]
pub struct HomeEventListener {
    delegate: EventListener,
}

impl<T> DataPoint<T> {
    pub fn map_value<U>(&self, f: impl FnOnce(&T) -> U) -> DataPoint<U> {
        let value = f(&self.value);
        DataPoint {
            value,
            timestamp: self.timestamp,
        }
    }
}

impl HomeEventListener {
    pub fn new(db_listener: PgListener) -> Self {
        Self {
            delegate: EventListener::new(db_listener, vec![api::THING_VALUE_ADDED_EVENT]),
        }
    }

    pub fn new_thing_value_added_listener(&self) -> Receiver<()> {
        self.delegate
            .new_listener(api::THING_VALUE_ADDED_EVENT)
            .unwrap()
    }

    pub async fn dispatch_events(self) -> Result<()> {
        self.delegate.dispatch_events().await
    }
}

impl HomeApi {
    pub fn new(db_pool: PgPool) -> Self {
        Self { db_pool }
    }

    pub async fn get_latest<'a, C: ChannelTypeInfo>(
        &self,
        id: &'a C,
    ) -> Result<DataPoint<C::ValueType>>
    where
        &'a C: Into<Channel>,
        C::ValueType: From<DbValue>,
    {
        let tag_id = get_tag_id(&self.db_pool, id.into(), false).await?;

        //TODO rewrite to max query
        let rec = sqlx::query!(
            r#"SELECT value as "value: DbValue", timestamp
            FROM THING_VALUES
            WHERE TAG_ID = $1
            ORDER BY timestamp DESC, id DESC
            LIMIT 1"#,
            tag_id
        )
        .fetch_optional(&self.db_pool)
        .await?;

        match rec {
            Some(r) => Ok(DataPoint {
                value: r.value.into(),
                timestamp: r.timestamp,
            }),
            None => anyhow::bail!("No data found"),
        }
    }

    pub async fn get_covering<'a, C: ChannelTypeInfo>(
        &self,
        id: &'a C,
        start: DateTime<Utc>,
    ) -> Result<Vec<DataPoint<C::ValueType>>>
    where
        &'a C: Into<Channel>,
        C::ValueType: From<DbValue>,
    {
        let tags_id = get_tag_id(&self.db_pool, id.into(), false).await?;

        //TODO rewrite to max query
        let rec = sqlx::query!(
            r#"(SELECT value as "value!: DbValue", timestamp as "timestamp!: DateTime<Utc>"
              FROM THING_VALUES
              WHERE TAG_ID = $1
              AND timestamp > $2)
            UNION ALL
            (SELECT value, timestamp
              FROM THING_VALUES
              WHERE TAG_ID = $1
              AND timestamp <= $2
              ORDER BY timestamp DESC
              LIMIT 1)"#,
            tags_id,
            start
        )
        .fetch_all(&self.db_pool)
        .await?;

        let dps: Vec<DataPoint<C::ValueType>> = rec
            .into_iter()
            .map(|row| DataPoint {
                value: C::ValueType::from(row.value),
                timestamp: row.timestamp,
            })
            .collect();

        Ok(dps)
    }

    pub async fn execute_command(&self, command: &Command, source: &CommandSource) -> Result<()> {
        let db_command = serde_json::json!(command);
        let (db_source_type, db_source_id): (DbCommandSource, String) = source.into();

        sqlx::query!(
            r#"INSERT INTO THING_COMMANDS (COMMAND, CREATED, STATUS, SOURCE_TYPE, SOURCE_ID) VALUES ($1, $2, $3, $4, $5)"#,
            db_command,
            chrono::Utc::now(),
            DbCommandState::Pending as DbCommandState,
            db_source_type as DbCommandSource,
            db_source_id
        )
        .execute(&self.db_pool)
        .await?;

        Ok(())
    }

    pub async fn get_all_commands_since<C: CommandId>(
        &self,
        target: C,
        since: DateTime<Utc>,
    ) -> Result<Vec<CommandExecution<C::CommandType>>> {
        let target: CommandTarget = target.into();
        let db_target = serde_json::json!(target);

        let records = sqlx::query!(
            r#"SELECT id, command, created, status as "status: DbCommandState", error, source_type as "source_type: DbCommandSource", source_id
                from THING_COMMANDS 
                where command @> $1 and created >= $2 
                order by created asc"#,
            db_target,
            since
        )
        .fetch_all(&self.db_pool)
        .await?;

        records
            .into_iter()
            .map(|row| {
                let source = CommandSource::from((row.source_type, row.source_id));
                Ok(CommandExecution {
                    id: row.id,
                    command: serde_json::from_value(row.command)?,
                    state: CommandState::from((row.status, row.error)),
                    created: row.created,
                    source,
                })
            })
            .collect()
    }

    //Can also be optimized to directly query the latest command
    pub async fn get_latest_command_since<C: CommandId>(
        &self,
        target: C,
        since: DateTime<Utc>,
    ) -> Result<Option<CommandExecution<C::CommandType>>> {
        let mut all_commands = self.get_all_commands_since(target, since).await?;
        Ok(all_commands.pop())
    }
}

#[cfg(test)]
mod get_all_commands_since {
    use super::*;
    use api::command::{PowerToggle, SetPower};
    use chrono::Duration;
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

        let api = HomeApi::new(db_pool);

        //WHEN
        let result = api
            .get_all_commands_since(
                PowerToggle::Dehumidifier,
                chrono::Utc::now() - Duration::minutes(8),
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

        let api = HomeApi::new(db_pool);

        //WHEN
        let result = api
            .get_all_commands_since(PowerToggle::Dehumidifier, t!(8 minutes ago))
            .await
            .unwrap();

        //THEN
        assert_eq!(result.len(), 0);
    }

    async fn insert_command<C: Into<Command> + Clone>(
        db_pool: &PgPool,
        command: &C,
        at: DateTime<Utc>,
    ) {
        let command: Command = command.clone().into();

        sqlx::query!(
            r#"INSERT INTO THING_COMMANDS (COMMAND, CREATED, STATUS, SOURCE_TYPE, SOURCE_ID) VALUES ($1, $2, $3, $4, $5)"#,
            serde_json::to_value(command).unwrap(),
            at,
            DbCommandState::Pending as DbCommandState,
            DbCommandSource::System as DbCommandSource,
            "unit-test".to_owned()
        )
        .execute(db_pool)
        .await
        .unwrap();
    }
}
