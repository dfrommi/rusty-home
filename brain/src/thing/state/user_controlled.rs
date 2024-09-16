use chrono::{Duration, Utc};

use crate::home_api;
use api::{
    command::{Command, CommandExecution, CommandTarget, PowerToggle},
    state::DataPoint,
};

use super::DataPointAccess;

pub enum UserControlled {
    Dehumidifier,
}

impl DataPointAccess<bool> for UserControlled {
    fn current_data_point(&self) -> crate::error::Result<DataPoint<bool>> {
        let state = super::Powered::Dehumidifier.current_data_point()?;
        let maybe_command =
            home_api().get_latest_command(&CommandTarget::SetPower(PowerToggle::Dehumidifier))?;

        tracing::debug!("command = {:?}", maybe_command);

        let user_controlled = match maybe_command {
            Some(CommandExecution {
                command: Command::SetPower { power_on, .. },
                created,
                ..
            }) => {
                let diff_to_command = state.timestamp - created;
                let diff_since_state_change = Utc::now() - state.timestamp;
                tracing::debug!(
                    "User controlled: diff_cmd = {:?} diff_state = {:?} / state = {:?}",
                    diff_to_command,
                    diff_since_state_change,
                    state
                );
                (diff_to_command > Duration::seconds(30)
                    && diff_since_state_change < Duration::minutes(15))
                    || power_on != state.value.is_on()
            }
            Some(c) => {
                tracing::error!("Returned command not matching query: {:?}", c);
                false
            }
            None => true,
        };

        Ok(DataPoint {
            value: user_controlled,
            timestamp: state.timestamp,
        })
    }
}
