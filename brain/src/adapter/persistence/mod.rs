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
use support::ext::ToOk;
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
        let db_source: DbCommandSource = source.into();

        sqlx::query!(
            r#"INSERT INTO THING_COMMANDS (COMMAND, TIMESTAMP, STATUS, SOURCE) VALUES ($1, $2, $3, $4)"#,
            db_command,
            chrono::Utc::now(),
            DbCommandState::Pending as DbCommandState,
            db_source as DbCommandSource
        )
        .execute(&self.db_pool)
        .await?;

        Ok(())
    }

    pub async fn get_latest_command_since<C: CommandId>(
        &self,
        target: C,
        since: DateTime<Utc>,
    ) -> Result<Option<CommandExecution<C::CommandType>>> {
        let target: CommandTarget = target.into();
        let db_target = serde_json::json!(target);

        let maybe_row = sqlx::query!(
            r#"SELECT id, command, timestamp, status as "status: DbCommandState", error, source as "source: DbCommandSource"
                from THING_COMMANDS 
                where command @> $1 and timestamp > $2 
                order by timestamp desc 
                limit 1"#,
            db_target,
            since
        )
        .fetch_optional(&self.db_pool)
        .await?;

        match maybe_row {
            Some(row) => Ok(Some(CommandExecution {
                id: row.id,
                command: serde_json::from_value(row.command)?,
                state: CommandState::from((row.status, row.error)),
                created: row.timestamp,
                source: row.source.into(),
            })),
            None => Ok(None),
        }
    }

    pub async fn is_latest_command_since(
        &self,
        command: impl Into<Command>,
        since: DateTime<Utc>,
        source: Option<CommandSource>,
    ) -> Result<bool> {
        let command: Command = command.into();
        let target = CommandTarget::from(&command);

        let result = self.get_latest_command_since(target, since).await?;

        match result {
            Some(row) => (source.is_none() || source == Some(row.source)) && row.command == command,
            None => false,
        }
        .to_ok()
    }
}

#[cfg(test)]
mod get_latest_command_since {
    use super::*;
    use api::command::{PowerToggle, SetPower};
    use chrono::Duration;

    #[sqlx::test(migrations = "../migrations")]
    async fn test_command_found(db_pool: PgPool) {
        let command = prepare_db(&db_pool).await;
        let api = HomeApi::new(db_pool);

        let result = api
            .get_latest_command_since(
                PowerToggle::Dehumidifier,
                chrono::Utc::now() - Duration::minutes(10),
            )
            .await
            .unwrap();

        assert!(result.is_some());
        assert_eq!(result.unwrap().command, command);
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn test_no_command(db_pool: PgPool) {
        prepare_db(&db_pool).await;
        let api = HomeApi::new(db_pool);

        let result = api
            .get_latest_command_since(
                PowerToggle::Dehumidifier,
                chrono::Utc::now() - Duration::minutes(2),
            )
            .await
            .unwrap();

        assert!(result.is_none());
    }

    async fn prepare_db(db_pool: &PgPool) -> SetPower {
        let command = SetPower {
            device: PowerToggle::Dehumidifier,
            power_on: true,
        };

        insert_command(db_pool, &command, Duration::minutes(5)).await;
        insert_command(
            db_pool,
            &SetPower {
                device: PowerToggle::LivingRoomNotificationLight,
                power_on: true,
            },
            Duration::minutes(3),
        )
        .await;

        command
    }

    async fn insert_command<C: Into<Command> + Clone>(
        db_pool: &PgPool,
        command: &C,
        ago: Duration,
    ) {
        let command: Command = command.clone().into();

        sqlx::query!(
            r#"INSERT INTO THING_COMMANDS (COMMAND, TIMESTAMP, STATUS, SOURCE) VALUES ($1, $2, $3, $4)"#,
            serde_json::to_value(command).unwrap(),
            chrono::Utc::now() - ago,
            DbCommandState::Pending as DbCommandState,
            DbCommandSource::System as DbCommandSource
        )
        .execute(db_pool)
        .await
        .unwrap();
    }
}
