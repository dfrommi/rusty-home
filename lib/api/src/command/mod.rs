use chrono::{DateTime, Utc};
use support::unit::DegreeCelsius;

pub mod db;

#[derive(Debug, Clone)]
pub enum Command {
    SetPower {
        item: PowerToggle,
        power_on: bool,
    },
    SetHeating {
        item: Thermostat,
        target_state: HeatingTargetState,
    },
}

pub enum CommandTarget {
    SetPower(PowerToggle),
    SetHeating(Thermostat),
}

#[derive(Debug, Clone)]
pub enum PowerToggle {
    Dehumidifier,
    LivingRoomNotificationLight,
}

#[derive(Debug, Clone)]
pub enum Thermostat {
    LivingRoom,
    Bedroom,
    Kitchen,
    RoomOfRequirements,
    Bathroom,
}

#[derive(Debug, Clone)]
pub enum HeatingTargetState {
    Off,
    Heat(DegreeCelsius),
}

#[derive(Debug)]
pub struct CommandExecution {
    pub id: i64,
    pub command: Command,
    pub state: CommandState,
    pub created: DateTime<Utc>,
    pub source: CommandSource,
}

#[derive(Debug)]
pub enum CommandState {
    Pending,
    InProgress,
    Success,
    Error(String),
}

#[derive(Debug, Eq, PartialEq)]
pub enum CommandSource {
    System,
    User,
}
