use anyhow::Result;
use api::{
    command::{
        db::schema::{DbCommandSource, DbCommandState},
        Command, CommandSource, CommandTarget,
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

    pub async fn is_latest_command_since(
        &self,
        command: &Command,
        since: DateTime<Utc>,
        source: Option<&CommandSource>,
    ) -> Result<bool> {
        let target: CommandTarget = command.into();
        let db_target = serde_json::json!(target);

        let row = sqlx::query!(
            r#"SELECT command, source as "source: DbCommandSource"
                from THING_COMMANDS 
                where command @> $1 and timestamp > $2 
                order by timestamp desc 
                limit 1"#,
            db_target,
            since
        )
        .fetch_optional(&self.db_pool)
        .await?;

        match row {
            Some(row) => {
                let returned_command: Command = serde_json::from_value(row.command)?;
                let returned_source: CommandSource = row.source.into();
                (source.is_none() || source == Some(&returned_source))
                    && &returned_command == command
            }
            None => false,
        }
        .to_ok()
    }
}
