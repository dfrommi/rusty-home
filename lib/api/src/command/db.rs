use sqlx::PgPool;

use crate::error::{Error, Result};

use self::schema::*;

use super::{Command, CommandExecution, CommandTarget};

pub async fn add_command(db_pool: &PgPool, command: &Command) -> Result<()> {
    let data: DbThingCommand = command.into();

    sqlx::query( "INSERT INTO THING_COMMANDS (TYPE, POSITION, PAYLOAD, TIMESTAMP, STATUS) VALUES ($1, $2, $3, $4, $5)")
            .bind(data.command_type)
            .bind(data.position)
            .bind(data.payload)
            .bind(chrono::Utc::now())
            .bind(DbCommandState::Pending)
            .execute(db_pool)
            .await?;

    Ok(())
}

pub async fn get_latest_for_target(
    db_pool: &PgPool,
    target: &CommandTarget,
) -> Result<Option<CommandExecution>> {
    let (command_type, device): (DbCommandType, DbDevice) = target.into();

    let row: Option<DbThingCommandRow> = sqlx::query_as(
        "SELECT *
            from THING_COMMANDS
            where type = $1
              and position = $2
           order by timestamp desc
           limit 1",
    )
    .bind(command_type)
    .bind(device)
    .fetch_optional(db_pool)
    .await?;

    Ok(match row {
        Some(row) => Option::Some(row.try_into()?),
        None => Option::None,
    })
}

//TODO handle too old commands -> expect TTL with command, store in DB and return error with message
pub async fn get_command_for_processing(db_pool: &PgPool) -> Result<Option<Command>> {
    let mut tx = db_pool.begin().await?;

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
            let maybe_command: Result<Command> = rec.data.try_into();

            let result = match maybe_command {
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
                Err(Error::LocationDataInconsistent) | Err(Error::Deserialisation(_)) => {
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
                Err(error) => return Err(error),
            };

            tx.commit().await?;
            Ok(result)
        }
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

mod schema {
    #[derive(serde::Serialize, serde::Deserialize)]
    pub struct SetPowerPayload {
        pub power_on: bool,
    }

    #[derive(Debug, sqlx::Type)]
    #[sqlx(type_name = "TEXT", rename_all = "SCREAMING_SNAKE_CASE")]
    pub enum DbCommandType {
        SetPower,
    }

    #[derive(Debug, sqlx::Type)]
    #[sqlx(type_name = "VARCHAR", rename_all = "SCREAMING_SNAKE_CASE")]
    pub enum DbCommandState {
        Pending,
        InProgress,
        Success,
        Error,
    }

    #[derive(Debug, Clone, PartialEq, Eq, Hash, sqlx::Type)]
    #[sqlx(type_name = "TEXT", rename_all = "SCREAMING_SNAKE_CASE")]
    pub enum DbDevice {
        Dehumidifier,
    }

    #[derive(Debug, sqlx::FromRow)]
    pub struct DbThingCommand {
        #[sqlx(rename = "type")]
        pub command_type: DbCommandType,
        pub position: DbDevice,
        pub payload: serde_json::Value,
    }

    #[derive(Debug, sqlx::FromRow)]
    pub struct DbThingCommandRow {
        pub id: i64,
        #[sqlx(flatten)]
        pub data: DbThingCommand,
        pub timestamp: chrono::DateTime<chrono::Utc>,
        pub status: DbCommandState,
        pub error: Option<String>,
    }
}

mod mapper {
    use serde_json::json;

    use crate::{
        command::{Command, CommandExecution, CommandState, CommandTarget, PowerToggle},
        error::Error,
    };

    use super::*;

    impl From<&Command> for DbThingCommand {
        fn from(val: &Command) -> Self {
            match val {
                Command::SetPower { item, power_on } => DbThingCommand {
                    command_type: DbCommandType::SetPower,
                    position: match item {
                        PowerToggle::Dehumidifier => DbDevice::Dehumidifier,
                    },
                    payload: json!(SetPowerPayload {
                        power_on: *power_on,
                    }),
                },
            }
        }
    }

    impl TryInto<Command> for DbThingCommand {
        type Error = Error;

        fn try_into(self) -> std::result::Result<Command, Self::Error> {
            let command = match self.command_type {
                DbCommandType::SetPower => Command::SetPower {
                    item: match self.position {
                        DbDevice::Dehumidifier => PowerToggle::Dehumidifier,
                        #[allow(unreachable_patterns)] //will be needed with more items
                        _ => return Err(Error::LocationDataInconsistent),
                    },
                    power_on: serde_json::from_value::<SetPowerPayload>(self.payload)?.power_on,
                },
            };

            Ok(command)
        }
    }

    impl TryInto<CommandExecution> for DbThingCommandRow {
        type Error = Error;

        fn try_into(self) -> std::result::Result<CommandExecution, Self::Error> {
            Ok(CommandExecution {
                id: self.id,
                command: self.data.try_into()?,
                state: match self.status {
                    DbCommandState::Pending => CommandState::Pending,
                    DbCommandState::InProgress => CommandState::InProgress,
                    DbCommandState::Success => CommandState::Success,
                    DbCommandState::Error => {
                        CommandState::Error(self.error.unwrap_or("unknown error".to_string()))
                    }
                },
                created: self.timestamp,
            })
        }
    }

    impl From<&CommandTarget> for (DbCommandType, DbDevice) {
        fn from(val: &CommandTarget) -> Self {
            match val {
                CommandTarget::SetPower(toggle) => (
                    DbCommandType::SetPower,
                    match toggle {
                        PowerToggle::Dehumidifier => DbDevice::Dehumidifier,
                    },
                ),
            }
        }
    }
}
