use self::schema::*;

pub mod schema {
    #[derive(Debug, Clone, sqlx::Type)]
    #[sqlx(type_name = "VARCHAR", rename_all = "snake_case")]
    pub enum DbCommandState {
        Pending,
        InProgress,
        Success,
        Error,
    }

    #[derive(Debug, Clone, PartialEq, Eq, sqlx::Type)]
    #[sqlx(type_name = "VARCHAR", rename_all = "snake_case")]
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
    use crate::command::{CommandSource, CommandState};

    impl From<(DbCommandState, Option<String>)> for CommandState {
        fn from((status, error): (DbCommandState, Option<String>)) -> Self {
            match status {
                DbCommandState::Pending => CommandState::Pending,
                DbCommandState::InProgress => CommandState::InProgress,
                DbCommandState::Success => CommandState::Success,
                DbCommandState::Error => {
                    CommandState::Error(error.unwrap_or("unknown error".to_string()))
                }
            }
        }
    }

    impl From<(DbCommandSource, String)> for CommandSource {
        fn from(value: (DbCommandSource, String)) -> Self {
            match value.0 {
                DbCommandSource::System => CommandSource::System(value.1),
                DbCommandSource::User => CommandSource::User(value.1),
            }
        }
    }

    impl From<&CommandSource> for (DbCommandSource, String) {
        fn from(val: &CommandSource) -> Self {
            match val {
                CommandSource::System(id) => (DbCommandSource::System, id.to_owned()),
                CommandSource::User(id) => (DbCommandSource::User, id.to_owned()),
            }
        }
    }
}
