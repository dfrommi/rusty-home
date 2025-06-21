use std::fmt::Display;

use api::{
    command::{Command, Fan},
    state::unit::{FanAirflow, FanSpeed},
};
use support::t;

use crate::core::planner::{CommandAction, ConditionalAction};

use super::{DataPointAccess, Opened};

#[derive(Debug, Clone)]
pub struct SupportVentilationWithFan {
    fan: Fan,
}

impl SupportVentilationWithFan {
    pub fn new(fan: Fan) -> Self {
        Self { fan }
    }
}

impl Display for SupportVentilationWithFan {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SupportVentilationWithFan[{:?}]", self.fan)
    }
}
impl CommandAction for SupportVentilationWithFan {
    fn command(&self) -> Command {
        Command::ControlFan {
            device: Fan::LivingRoomCeilingFan,
            speed: FanAirflow::Forward(FanSpeed::Low),
        }
    }

    fn source(&self) -> api::command::CommandSource {
        super::action_source(self)
    }
}

impl<T> ConditionalAction<T> for SupportVentilationWithFan
where
    T: DataPointAccess<Opened>,
{
    async fn preconditions_fulfilled(&self, api: &T) -> anyhow::Result<bool> {
        let window = match self.fan {
            Fan::LivingRoomCeilingFan => Opened::LivingRoomWindowOrDoor,
        };

        let opened_dp = api.current_data_point(window).await?;
        let elapsed = opened_dp.timestamp.elapsed();

        if !opened_dp.value {
            return Ok(false);
        }

        if elapsed < t!(1 minutes) {
            tracing::trace!("Window is open, but for less than a minute");
            return Ok(false);
        } else if elapsed > t!(10 minutes) {
            tracing::trace!(
                "Window is open, but for more than 10 minutes. Stopping active support"
            );
            return Ok(false);
        }

        Ok(true)
    }
}
