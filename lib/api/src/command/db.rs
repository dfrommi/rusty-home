use self::schema::*;

pub mod schema {
    #[derive(Debug, Clone, sqlx::Type)]
    #[sqlx(type_name = "VARCHAR", rename_all = "SCREAMING_SNAKE_CASE")]
    pub enum DbCommandState {
        Pending,
        InProgress,
        Success,
        Error,
    }

    #[derive(Debug, Clone, PartialEq, Eq, sqlx::Type)]
    #[sqlx(type_name = "VARCHAR", rename_all = "SCREAMING_SNAKE_CASE")]
    pub enum DbCommandSource {
        System,
        User,
    }

    #[derive(Debug, Clone, sqlx::FromRow)]
    pub struct DbThingCommandRow {
        pub id: i64,
        pub command: serde_json::Value,
        pub timestamp: chrono::DateTime<chrono::Utc>,
        pub status: DbCommandState,
        pub error: Option<String>,
        pub source: DbCommandSource,
    }
}

pub mod mapper {
    use super::*;
    use crate::command::{CommandExecution, CommandSource, CommandState};

    impl TryInto<CommandExecution> for DbThingCommandRow {
        type Error = anyhow::Error;

        fn try_into(self) -> std::result::Result<CommandExecution, Self::Error> {
            Ok(CommandExecution {
                id: self.id,
                command: serde_json::from_value(self.command)?,
                state: match self.status {
                    DbCommandState::Pending => CommandState::Pending,
                    DbCommandState::InProgress => CommandState::InProgress,
                    DbCommandState::Success => CommandState::Success,
                    DbCommandState::Error => {
                        CommandState::Error(self.error.unwrap_or("unknown error".to_string()))
                    }
                },
                created: self.timestamp,
                source: self.source.into(),
            })
        }
    }

    impl From<DbCommandSource> for CommandSource {
        fn from(value: DbCommandSource) -> Self {
            match value {
                DbCommandSource::System => CommandSource::System,
                DbCommandSource::User => CommandSource::User,
            }
        }
    }

    impl From<&CommandSource> for DbCommandSource {
        fn from(val: &CommandSource) -> Self {
            match val {
                CommandSource::System => DbCommandSource::System,
                CommandSource::User => DbCommandSource::User,
            }
        }
    }
}
