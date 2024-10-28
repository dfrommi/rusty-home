use std::fmt::Display;

use anyhow::{Ok, Result};
use api::command::Command;
use chrono::{Duration, Utc};
use support::{time::DailyTimeRange, unit::DegreeCelsius};

use crate::{
    adapter::persistence::DataPoint,
    thing::{
        AutomaticTemperatureIncrease, ColdAirComingIn, DataPointAccess, Executable, Opened,
        Resident, ResidentState,
    },
};

use super::{Action, HeatingZone, Resource};

#[derive(Debug, Clone)]
pub struct NoHeatingDuringVentilation {
    heating_zone: HeatingZone,
}

impl NoHeatingDuringVentilation {
    pub fn new(heating_zone: HeatingZone) -> Self {
        Self { heating_zone }
    }
}

#[derive(Debug, Clone)]
pub struct NoHeatingDuringAutomaticTemperatureIncrease {
    heating_zone: HeatingZone,
}

impl NoHeatingDuringAutomaticTemperatureIncrease {
    pub fn new(heating_zone: HeatingZone) -> Self {
        Self { heating_zone }
    }
}

#[derive(Debug, Clone)]
pub struct ExtendHeatingUntilSleeping {
    heating_zone: HeatingZone,
    target_temperature: DegreeCelsius,
    time_range: DailyTimeRange,
}

#[derive(Debug, Clone)]
pub struct DeferHeatingUntilVentilationDone {
    heating_zone: HeatingZone,
    target_temperature: DegreeCelsius,
    time_range: DailyTimeRange,
}

impl ExtendHeatingUntilSleeping {
    pub fn new(
        heating_zone: HeatingZone,
        target_temperature: DegreeCelsius,
        start_hm: (u32, u32),
        latest_until_hm: (u32, u32),
    ) -> Self {
        Self {
            heating_zone,
            target_temperature,
            time_range: DailyTimeRange::new(start_hm, latest_until_hm),
        }
    }

    async fn is_matching_target(&self) -> Result<DataPoint<bool>> {
        let (set_point, auto_mode) = (
            self.heating_zone.current_set_point(),
            self.heating_zone.auto_mode(),
        );

        let (set_point, auto_mode) = tokio::try_join!(
            set_point.current_data_point(),
            auto_mode.current_data_point()
        )?;

        Ok(DataPoint {
            value: set_point.value == self.target_temperature && auto_mode.value,
            timestamp: std::cmp::max(set_point.timestamp, auto_mode.timestamp),
        })
    }
}

impl Display for ExtendHeatingUntilSleeping {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "ExtendHeatingUntilSleeping[{} -> {} ({})]",
            self.heating_zone, self.target_temperature, self.time_range
        )
    }
}

impl DeferHeatingUntilVentilationDone {
    pub fn new(
        heating_zone: HeatingZone,
        target_temperature: DegreeCelsius,
        start_hm: (u32, u32),
        latest_until_hm: (u32, u32),
    ) -> Self {
        Self {
            heating_zone,
            target_temperature,
            time_range: DailyTimeRange::new(start_hm, latest_until_hm),
        }
    }

    fn window(&self) -> Opened {
        match self.heating_zone {
            HeatingZone::LivingRoom => Opened::LivingRoomWindowOrDoor,
            HeatingZone::Bedroom => Opened::BedroomWindow,
            HeatingZone::Kitchen => Opened::KitchenWindow,
            HeatingZone::RoomOfRequirements => Opened::LivingRoomWindowOrDoor,
            HeatingZone::Bathroom => Opened::BedroomWindow,
        }
    }
}

impl Action for ExtendHeatingUntilSleeping {
    async fn preconditions_fulfilled(&self) -> Result<bool> {
        if !self.time_range.contains(Utc::now()) {
            return Ok(false);
        }

        let (dennis, sabine) =
            tokio::try_join!(Resident::Dennis.current(), Resident::Sabine.current(),)?;

        //TODO more granular (one still or already up)
        //home = not away or sleeping
        Ok(dennis == ResidentState::Home || sabine == ResidentState::Home)
    }

    async fn is_running(&self) -> Result<bool> {
        let matches_target = self.is_matching_target().await?;

        //TODO check command already sent within last 2 minutes
        Ok(matches_target.value && self.time_range.contains(matches_target.timestamp))
    }

    async fn start(&self) -> Result<()> {
        Command::SetHeating {
            device: self.heating_zone.thermostat(),
            target_state: api::command::HeatingTargetState::Heat {
                temperature: self.target_temperature,
                until: self.time_range.for_today().1,
            },
        }
        .execute()
        .await
    }

    async fn stop(&self) -> Result<()> {
        Command::SetHeating {
            device: self.heating_zone.thermostat(),
            target_state: api::command::HeatingTargetState::Auto,
        }
        .execute()
        .await
    }

    fn controls_resource(&self) -> Option<Resource> {
        Some(self.heating_zone.resource())
    }
}

impl Action for DeferHeatingUntilVentilationDone {
    async fn preconditions_fulfilled(&self) -> Result<bool> {
        if !self.time_range.contains(Utc::now()) {
            return Ok(false);
        }

        let window_opened = self.window().current_data_point().await?;
        Ok(!self.time_range.contains(window_opened.timestamp))
    }

    async fn is_running(&self) -> Result<bool> {
        let has_expected_manual_heating =
            is_manual_heating_to(&self.heating_zone, self.target_temperature).await?;

        Ok(has_expected_manual_heating.value
            && self
                .time_range
                .contains(has_expected_manual_heating.timestamp))
    }

    async fn start(&self) -> Result<()> {
        Command::SetHeating {
            device: self.heating_zone.thermostat(),
            target_state: api::command::HeatingTargetState::Heat {
                temperature: self.target_temperature,
                until: self.time_range.for_today().1,
            },
        }
        .execute()
        .await
    }

    async fn stop(&self) -> Result<()> {
        Command::SetHeating {
            device: self.heating_zone.thermostat(),
            target_state: api::command::HeatingTargetState::Auto,
        }
        .execute()
        .await
    }

    fn controls_resource(&self) -> Option<Resource> {
        Some(self.heating_zone.resource())
    }
}

impl Display for DeferHeatingUntilVentilationDone {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "DeferHeatingUntilVentilationDone[{} -> {} ({})]",
            self.heating_zone, self.target_temperature, self.time_range
        )
    }
}

impl Action for NoHeatingDuringVentilation {
    async fn preconditions_fulfilled(&self) -> Result<bool> {
        match self.heating_zone {
            HeatingZone::LivingRoom => ColdAirComingIn::LivingRoom,
            HeatingZone::Bedroom => ColdAirComingIn::Bedroom,
            HeatingZone::Kitchen => ColdAirComingIn::Kitchen,
            HeatingZone::RoomOfRequirements => ColdAirComingIn::RoomOfRequirements,
            HeatingZone::Bathroom => ColdAirComingIn::Bedroom,
        }
        .current()
        .await
    }

    async fn is_running(&self) -> Result<bool> {
        self.heating_zone
            .current_set_point()
            .current()
            .await
            .map(|v| v == DegreeCelsius(0.0))
    }

    async fn start(&self) -> Result<()> {
        Command::SetHeating {
            device: self.heating_zone.thermostat(),
            target_state: api::command::HeatingTargetState::Off,
        }
        .execute()
        .await
    }

    async fn stop(&self) -> Result<()> {
        Command::SetHeating {
            device: self.heating_zone.thermostat(),
            target_state: api::command::HeatingTargetState::Auto,
        }
        .execute()
        .await
    }

    fn controls_resource(&self) -> Option<Resource> {
        Some(self.heating_zone.resource())
    }
}

impl Display for NoHeatingDuringVentilation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "NoHeatingDuringVentilation[{}]", self.heating_zone)
    }
}

impl Action for NoHeatingDuringAutomaticTemperatureIncrease {
    async fn preconditions_fulfilled(&self) -> Result<bool> {
        match self.heating_zone {
            HeatingZone::LivingRoom => AutomaticTemperatureIncrease::LivingRoom,
            HeatingZone::Bedroom => AutomaticTemperatureIncrease::Bedroom,
            HeatingZone::Kitchen => AutomaticTemperatureIncrease::Kitchen,
            HeatingZone::RoomOfRequirements => AutomaticTemperatureIncrease::RoomOfRequirements,
            HeatingZone::Bathroom => AutomaticTemperatureIncrease::Bedroom,
        }
        .current()
        .await
    }

    async fn is_running(&self) -> Result<bool> {
        self.heating_zone
            .current_set_point()
            .current()
            .await
            .map(|v| v == DegreeCelsius(7.1))
    }

    async fn start(&self) -> Result<()> {
        Command::SetHeating {
            device: self.heating_zone.thermostat(),
            target_state: api::command::HeatingTargetState::Heat {
                temperature: DegreeCelsius(7.1),
                until: Utc::now() + Duration::hours(1),
            },
        }
        .execute()
        .await
    }

    async fn stop(&self) -> Result<()> {
        Command::SetHeating {
            device: self.heating_zone.thermostat(),
            target_state: api::command::HeatingTargetState::Auto,
        }
        .execute()
        .await
    }

    fn controls_resource(&self) -> Option<Resource> {
        Some(self.heating_zone.resource())
    }
}

impl Display for NoHeatingDuringAutomaticTemperatureIncrease {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "NoHeatingDuringAutomaticTemperatureIncrease[{}]",
            self.heating_zone
        )
    }
}

async fn is_manual_heating_to(
    heating_zone: &HeatingZone,
    target_temperature: DegreeCelsius,
) -> Result<DataPoint<bool>> {
    let (set_point, auto_mode) = (heating_zone.current_set_point(), heating_zone.auto_mode());

    let (set_point, auto_mode) = tokio::try_join!(
        set_point.current_data_point(),
        auto_mode.current_data_point()
    )?;

    Ok(DataPoint {
        value: set_point.value == target_temperature && !auto_mode.value,
        timestamp: std::cmp::max(set_point.timestamp, auto_mode.timestamp),
    })
}
