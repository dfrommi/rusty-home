use chrono::{DateTime, Utc};

pub(super) mod db;

#[derive(Debug)]
pub enum PowerToggle {
    Dehumidifier,
}

#[derive(Debug)]
pub enum Command {
    SetPower { item: PowerToggle, power_on: bool },
}

pub enum CommandTarget {
    SetPower(PowerToggle),
}

#[derive(Debug)]
pub struct CommandExecution {
    pub id: i64,
    pub command: Command,
    pub state: CommandState,
    pub created: DateTime<Utc>,
}

#[derive(Debug)]
pub enum CommandState {
    Pending,
    InProgress,
    Success,
    Error(String),
}
