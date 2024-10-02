use api::{
    command::{
        db::schema::{DbCommandState, DbThingCommandRow},
        Command,
    },
    get_tag_id,
    state::ChannelValue,
};
use chrono::{DateTime, Utc};
use sqlx::PgPool;

pub use crate::error::Result;

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
                order by TIMESTAMP ASC
                limit 1
                for update skip locked",
        )
        .bind(DbCommandState::Pending)
        .fetch_optional(&mut *tx)
        .await?;

        match maybe_rec {
            None => Ok(None),
            Some(rec) => {
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

//TODO error message
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
