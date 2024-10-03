use api::{
    command::{
        db::schema::{DbCommandState, DbCommandType, DbDevice, DbThingCommandRow},
        Command,
    },
    get_tag_id,
    state::ChannelValue,
    EventListener,
};
use chrono::{DateTime, Utc};
use sqlx::{postgres::PgListener, PgPool};
use tokio::sync::broadcast::Receiver;

use crate::error::Error;
pub use crate::error::Result;

#[derive(Debug)]
pub struct BackendEventListener {
    delegate: EventListener,
}

impl BackendEventListener {
    pub fn new(db_listener: PgListener) -> Self {
        Self {
            delegate: EventListener::new(db_listener, vec![api::THING_COMMAND_ADDED_EVENT]),
        }
    }

    pub fn new_command_added_listener(&self) -> Receiver<()> {
        self.delegate
            .new_listener(api::THING_COMMAND_ADDED_EVENT)
            .unwrap()
    }

    pub async fn dispatch_events(self) -> Result<()> {
        self.delegate.dispatch_events().await.map_err(Error::Api)
    }
}

#[derive(Debug, Clone)]
pub struct BackendApi {
    db_pool: PgPool,
}

impl BackendApi {
    pub fn new(db_pool: PgPool) -> Self {
        Self { db_pool }
    }

    //TODO handle too old commands -> expect TTL with command, store in DB and return error with message
    pub async fn get_command_for_processing(&self) -> Result<Option<Command>> {
        let mut tx = self.db_pool.begin().await?;

        let maybe_rec: Option<DbThingCommandRow> = sqlx::query_as(
            "SELECT * 
                from THING_COMMANDS 
                where status = $1
                order by TIMESTAMP DESC
                limit 1
                for update skip locked",
        )
        .bind(DbCommandState::Pending)
        .fetch_optional(&mut *tx)
        .await?;

        match maybe_rec {
            None => Ok(None),
            Some(rec) => {
                mark_other_commands_superseeded(
                    &mut *tx,
                    rec.id,
                    &rec.data.command_type,
                    &rec.data.device,
                )
                .await?;

                let result = match rec.data.try_into() {
                    Ok(command) => {
                        set_command_status_in_tx(
                            &mut *tx,
                            rec.id,
                            DbCommandState::InProgress,
                            Option::None,
                        )
                        .await?;
                        Some(command)
                    }
                    Err(api::Error::LocationDataInconsistent)
                    | Err(api::Error::Deserialisation(_)) => {
                        //TODO error message
                        set_command_status_in_tx(
                            &mut *tx,
                            rec.id,
                            DbCommandState::Error,
                            Option::Some("Error in format of stored command"),
                        )
                        .await?;
                        None
                    }
                    Err(error) => return Err(error.into()),
                };

                tx.commit().await?;
                Ok(result)
            }
        }
    }

    pub async fn add_thing_value(
        &self,
        value: &ChannelValue,
        timestamp: &DateTime<Utc>,
    ) -> Result<()> {
        let tags_id = get_tag_id(&self.db_pool, value.into(), true).await?;

        let fvalue: f64 = value.into();

        sqlx::query(
            "WITH latest_value AS (
                SELECT value
                FROM thing_values
                WHERE tag_id = $1
                ORDER BY timestamp DESC
                LIMIT 1
            )
            INSERT INTO thing_values (tag_id, value, timestamp)
            SELECT $1, $2, $3
            WHERE NOT EXISTS ( SELECT 1 FROM latest_value WHERE value = $2)",
        )
        .bind(tags_id)
        .bind(fvalue)
        .bind(timestamp)
        .execute(&self.db_pool)
        .await?;

        //info!("Inserted new value: {:?}", event);

        Ok(())
    }
}

async fn set_command_status_in_tx(
    executor: impl sqlx::Executor<'_, Database = sqlx::Postgres>,
    command_id: i64,
    status: DbCommandState,
    error_message: Option<&str>,
) -> std::result::Result<(), sqlx::Error> {
    sqlx::query("UPDATE THING_COMMANDS SET status = $2, error = $3 WHERE id = $1")
        .bind(command_id)
        .bind(status)
        .bind(error_message)
        .execute(executor)
        .await
        .map(|_| ())
}

async fn mark_other_commands_superseeded(
    executor: impl sqlx::Executor<'_, Database = sqlx::Postgres>,
    excluded_command_id: i64,
    command_type: &DbCommandType,
    device: &DbDevice,
) -> std::result::Result<(), sqlx::Error> {
    sqlx::query("UPDATE THING_COMMANDS SET status = $1, error = $2 WHERE NOT id = $3 AND status = $4 AND type = $5 AND device = $6")
        .bind(DbCommandState::Error)
        .bind(format!("Command was superseeded by {}", excluded_command_id))
        .bind(excluded_command_id)
        .bind(DbCommandState::Pending)
        .bind(command_type)
        .bind(device)
        .execute(executor)
        .await
        .map(|_| ())
}
