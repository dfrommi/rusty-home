use self::schema::*;

pub mod schema {
    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
    pub struct DbSetPowerPayload {
        pub power_on: bool,
    }

    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
    pub struct DbSetHeatingPayload {
        pub target_temperature: f64,
    }

    #[derive(Debug, Clone, sqlx::Type)]
    #[sqlx(type_name = "TEXT", rename_all = "SCREAMING_SNAKE_CASE")]
    pub enum DbCommandType {
        SetPower,
        SetHeating,
    }

    #[derive(Debug, Clone, sqlx::Type)]
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
        LivingRoomNotificationLight,
        LivingRoomThermostat,
        BedroomThermostat,
        KitchenThermostat,
        RoomOfRequirementsThermostat,
        BathroomThermostat,
    }

    #[derive(Debug, Clone, sqlx::Type)]
    #[sqlx(type_name = "VARCHAR", rename_all = "SCREAMING_SNAKE_CASE")]
    pub enum DbCommandSource {
        System,
        User,
    }

    #[derive(Debug, Clone, sqlx::FromRow)]
    pub struct DbThingCommand {
        #[sqlx(rename = "type")]
        pub command_type: DbCommandType,
        pub device: DbDevice,
        pub payload: serde_json::Value,
    }

    #[derive(Debug, Clone, sqlx::FromRow)]
    pub struct DbThingCommandRow {
        pub id: i64,
        #[sqlx(flatten)]
        pub data: DbThingCommand,
        pub timestamp: chrono::DateTime<chrono::Utc>,
        pub status: DbCommandState,
        pub error: Option<String>,
        pub source: DbCommandSource,
    }
}

pub mod mapper {
    use anyhow::bail;
    use serde_json::json;
    use support::unit::DegreeCelsius;

    use crate::command::{
        Command, CommandExecution, CommandSource, CommandState, CommandTarget, HeatingTargetState,
        PowerToggle, Thermostat,
    };

    use super::*;

    impl From<&Command> for DbThingCommand {
        fn from(val: &Command) -> Self {
            match val {
                Command::SetPower { item, power_on } => DbThingCommand {
                    command_type: DbCommandType::SetPower,
                    device: match item {
                        PowerToggle::Dehumidifier => DbDevice::Dehumidifier,
                        PowerToggle::LivingRoomNotificationLight => {
                            DbDevice::LivingRoomNotificationLight
                        }
                    },
                    payload: json!(DbSetPowerPayload {
                        power_on: *power_on,
                    }),
                },
                Command::SetHeating { item, target_state } => DbThingCommand {
                    command_type: DbCommandType::SetHeating,
                    device: match item {
                        Thermostat::LivingRoom => DbDevice::LivingRoomThermostat,
                        Thermostat::Bedroom => DbDevice::BedroomThermostat,
                        Thermostat::Kitchen => DbDevice::KitchenThermostat,
                        Thermostat::RoomOfRequirements => DbDevice::RoomOfRequirementsThermostat,
                        Thermostat::Bathroom => DbDevice::BathroomThermostat,
                    },
                    payload: json!(DbSetHeatingPayload {
                        target_temperature: match target_state {
                            HeatingTargetState::Off => 0.0,
                            HeatingTargetState::Heat(degree_celsius) => degree_celsius.into(),
                        }
                    }),
                },
            }
        }
    }

    impl TryInto<Command> for DbThingCommand {
        type Error = anyhow::Error;

        fn try_into(self) -> std::result::Result<Command, Self::Error> {
            let command = match self.command_type {
                DbCommandType::SetPower => Command::SetPower {
                    item: match self.device {
                        DbDevice::Dehumidifier => PowerToggle::Dehumidifier,
                        DbDevice::LivingRoomNotificationLight => {
                            PowerToggle::LivingRoomNotificationLight
                        }

                        _ => bail!(
                            "Combination of command type {:?} and device {:?} not supported",
                            self.command_type,
                            self.device
                        ),
                    },
                    power_on: serde_json::from_value::<DbSetPowerPayload>(self.payload)?.power_on,
                },
                DbCommandType::SetHeating => Command::SetHeating {
                    item: match self.device {
                        DbDevice::LivingRoomThermostat => Thermostat::LivingRoom,
                        DbDevice::BedroomThermostat => Thermostat::Bedroom,
                        DbDevice::KitchenThermostat => Thermostat::Kitchen,
                        DbDevice::RoomOfRequirementsThermostat => Thermostat::RoomOfRequirements,
                        DbDevice::BathroomThermostat => Thermostat::Bathroom,

                        _ => bail!(
                            "Combination of command type {:?} and device {:?} not supported",
                            self.command_type,
                            self.device
                        ),
                    },

                    target_state: {
                        let payload: DbSetHeatingPayload = serde_json::from_value(self.payload)?;
                        if payload.target_temperature == 0.0 {
                            HeatingTargetState::Off
                        } else {
                            HeatingTargetState::Heat(DegreeCelsius(payload.target_temperature))
                        }
                    },
                },
            };

            Ok(command)
        }
    }

    impl From<&CommandTarget> for (DbCommandType, DbDevice) {
        fn from(val: &CommandTarget) -> Self {
            match val {
                CommandTarget::SetPower(toggle) => (
                    DbCommandType::SetPower,
                    match toggle {
                        PowerToggle::Dehumidifier => DbDevice::Dehumidifier,
                        PowerToggle::LivingRoomNotificationLight => {
                            DbDevice::LivingRoomNotificationLight
                        }
                    },
                ),
                CommandTarget::SetHeating(thermostat) => (
                    DbCommandType::SetHeating,
                    match thermostat {
                        Thermostat::LivingRoom => DbDevice::LivingRoomThermostat,
                        Thermostat::Bedroom => DbDevice::BedroomThermostat,
                        Thermostat::Kitchen => DbDevice::KitchenThermostat,
                        Thermostat::RoomOfRequirements => DbDevice::RoomOfRequirementsThermostat,
                        Thermostat::Bathroom => DbDevice::BathroomThermostat,
                    },
                ),
            }
        }
    }

    impl TryInto<CommandExecution> for DbThingCommandRow {
        type Error = anyhow::Error;

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
                source: match self.source {
                    DbCommandSource::System => CommandSource::System,
                    DbCommandSource::User => CommandSource::User,
                },
            })
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
