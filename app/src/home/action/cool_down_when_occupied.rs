use std::fmt::Display;

use crate::core::{HomeApi, planner::SimpleAction, timeseries::DataPoint, unit::DegreeCelsius};
use crate::home::command::{Command, CommandSource, Fan};
use crate::home::state::{FanAirflow, FanSpeed, Temperature};
use crate::t;

use super::{DataPointAccess as _, Resident, trigger_once_and_keep_running};

#[derive(Debug, Clone)]
pub enum CoolDownWhenOccupied {
    Bedroom,
    LivingRoom,
}

impl Display for CoolDownWhenOccupied {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "CoolDownWhenOccupied[{}]",
            match self {
                CoolDownWhenOccupied::Bedroom => "Bedroom",
                CoolDownWhenOccupied::LivingRoom => "LivingRoom",
            }
        )
    }
}

impl SimpleAction for CoolDownWhenOccupied {
    fn command(&self) -> Command {
        Command::ControlFan {
            device: self.fan(),
            speed: FanAirflow::Forward(FanSpeed::Low),
        }
    }

    fn source(&self) -> CommandSource {
        super::action_source(self)
    }

    async fn preconditions_fulfilled(&self, api: &HomeApi) -> anyhow::Result<bool> {
        let temperature = self.temperature().current(api).await?;
        tracing::info!(temperature = ?temperature, "Cooling down temperature is {}", temperature);

        if temperature < DegreeCelsius(26.5) {
            tracing::trace!(
                temperature = ?temperature,
                "No cooldown needed, because temperature is below 26.5°C at {}", temperature
            );
            return Ok(false);
        }

        match self {
            CoolDownWhenOccupied::Bedroom => self.trigger_when_sleeping(api).await,
            CoolDownWhenOccupied::LivingRoom => self.trigger_when_on_couch(api).await,
        }
    }
}

impl CoolDownWhenOccupied {
    fn temperature(&self) -> Temperature {
        match self {
            CoolDownWhenOccupied::Bedroom => Temperature::BedroomDoor,
            CoolDownWhenOccupied::LivingRoom => Temperature::LivingRoomDoor,
        }
    }

    fn fan(&self) -> Fan {
        match self {
            CoolDownWhenOccupied::Bedroom => Fan::BedroomCeilingFan,
            CoolDownWhenOccupied::LivingRoom => Fan::LivingRoomCeilingFan,
        }
    }

    async fn trigger_when_sleeping(&self, api: &HomeApi) -> anyhow::Result<bool> {
        let anyone_sleeping = {
            let dennis_sleeping = Resident::DennisSleeping.current_data_point(api).await?;
            let sabine_sleeping = Resident::SabineSleeping.current_data_point(api).await?;

            match (dennis_sleeping.value, sabine_sleeping.value) {
                (false, false) => {
                    DataPoint::new(false, std::cmp::min(dennis_sleeping.timestamp, sabine_sleeping.timestamp))
                }
                (true, false) => dennis_sleeping,
                (false, true) => sabine_sleeping,
                (true, true) => {
                    DataPoint::new(true, std::cmp::min(dennis_sleeping.timestamp, sabine_sleeping.timestamp))
                }
            }
        };

        //anyone sleeping?
        if !anyone_sleeping.value {
            tracing::trace!("No cooldown needed, because nobody is sleeping");
            return Ok(false);
        }

        trigger_once_and_keep_running(&self.command(), &self.source(), anyone_sleeping.timestamp, api).await
    }

    async fn trigger_when_on_couch(&self, api: &HomeApi) -> anyhow::Result<bool> {
        let on_couch = Resident::AnyoneOnCouch.current_data_point(api).await?;
        let time_since_change = on_couch.timestamp.elapsed();

        if !on_couch.value && time_since_change > t!(5 minutes) {
            tracing::trace!("No cooldown needed, because nobody is on the couch for {}", time_since_change);
            return Ok(false);
        }

        if on_couch.value && time_since_change < t!(1 minutes) {
            tracing::trace!(
                "No cooldown needed yet, because couch is occupied for less than a minute ({})",
                time_since_change
            );
            return Ok(false);
        }

        trigger_once_and_keep_running(&self.command(), &self.source(), on_couch.timestamp, api).await
    }
}
