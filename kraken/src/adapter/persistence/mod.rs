use api::{
    command::{
        db::schema::{DbCommandSource, DbCommandState, DbThingCommandRow},
        CommandExecution,
    },
    get_tag_id,
    state::{db::DbValue, ChannelValue},
    EventListener,
};
use chrono::{DateTime, Utc};
use sqlx::{postgres::PgListener, PgPool};
use tokio::sync::broadcast::Receiver;

use anyhow::Result;

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
        self.delegate.dispatch_events().await
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
    pub async fn get_command_for_processing(&self) -> Result<Option<CommandExecution>> {
        let mut tx = self.db_pool.begin().await?;

        let maybe_rec = sqlx::query_as!(
            DbThingCommandRow,
            r#"SELECT id, command, timestamp, status as "status: DbCommandState", error, source as "source: DbCommandSource"
                from THING_COMMANDS 
                where status = $1
                order by TIMESTAMP DESC
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

                let result: Option<CommandExecution> =
                    match TryInto::<CommandExecution>::try_into(rec) {
                        Ok(command) => {
                            set_command_status_in_tx(
                                &mut *tx,
                                id,
                                DbCommandState::InProgress,
                                Option::None,
                            )
                            .await?;

                            Some(command)
                        }
                        Err(e) => {
                            set_command_status_in_tx(
                                &mut *tx,
                                id,
                                DbCommandState::Error,
                                Option::Some(
                                    format!("Error reading stored command: {}", e).as_str(),
                                ),
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

    pub async fn add_thing_value(
        &self,
        value: &ChannelValue,
        timestamp: &DateTime<Utc>,
    ) -> Result<()> {
        let tags_id = get_tag_id(&self.db_pool, value.into(), true).await?;

        let fvalue: DbValue = value.into();

        sqlx::query!(
            r#"WITH latest_value AS (
                SELECT value
                FROM thing_values
                WHERE tag_id = $1
                ORDER BY timestamp DESC, id DESC
                LIMIT 1
            )
            INSERT INTO thing_values (tag_id, value, timestamp)
            SELECT $1, $2, $3
            WHERE NOT EXISTS ( SELECT 1 FROM latest_value WHERE value = $2)"#,
            tags_id,
            fvalue.as_ref(),
            timestamp
        )
        .execute(&self.db_pool)
        .await?;

        //info!("Inserted new value: {:?}", event);

        Ok(())
    }

    pub async fn set_command_state_success(&self, command_id: i64) -> Result<()> {
        set_command_status_in_tx(&self.db_pool, command_id, DbCommandState::Success, None).await
    }

    pub async fn set_command_state_error(
        &self,
        command_id: i64,
        error_message: &str,
    ) -> Result<()> {
        set_command_status_in_tx(
            &self.db_pool,
            command_id,
            DbCommandState::Error,
            Some(error_message),
        )
        .await
    }
}

async fn set_command_status_in_tx(
    executor: impl sqlx::Executor<'_, Database = sqlx::Postgres>,
    command_id: i64,
    status: DbCommandState,
    error_message: Option<&str>,
) -> Result<()> {
    sqlx::query!(
        r#"UPDATE THING_COMMANDS SET status = $2, error = $3 WHERE id = $1"#,
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
            SELECT command->'type' as type, command->'device' as device FROM THING_COMMANDS WHERE id = $1
        )
        UPDATE THING_COMMANDS
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
