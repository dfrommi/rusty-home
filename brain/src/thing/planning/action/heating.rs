use anyhow::Result;
use api::command::Command;
use support::unit::DegreeCelsius;

use crate::thing::{AutomaticTemperatureIncrease, ColdAirComingIn, DataPointAccess, Executable};

use super::{Action, HeatingZone, Resource};

#[derive(Debug, Clone)]
pub struct Heat {
    heating_zone: HeatingZone,
    target_temperature: DegreeCelsius,
}

impl Heat {
    pub fn new(heating_zone: HeatingZone, target_temperature: DegreeCelsius) -> Self {
        Self {
            heating_zone,
            target_temperature,
        }
    }
}

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

impl Action for Heat {
    async fn preconditions_fulfilled(&self) -> Result<bool> {
        Ok(true)
    }

    async fn is_running(&self) -> Result<bool> {
        self.heating_zone
            .current_set_point()
            .current()
            .await
            .map(|current| current == self.target_temperature)
    }

    async fn is_user_controlled(&self) -> Result<bool> {
        self.heating_zone.user_controlled().current().await
    }

    async fn start(&self) -> Result<()> {
        Command::SetHeating {
            device: self.heating_zone.thermostat(),
            target_state: api::command::HeatingTargetState::Heat {
                temperature: self.target_temperature,
            },
        }
        .execute()
        .await
    }

    async fn stop(&self) -> Result<()> {
        Command::SetHeating {
            device: self.heating_zone.thermostat(),
            target_state: api::command::HeatingTargetState::Off,
        }
        .execute()
        .await
    }

    fn controls_resource(&self) -> Option<Resource> {
        Some(self.heating_zone.resource())
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

    async fn is_user_controlled(&self) -> Result<bool> {
        Ok(false) //no user override possible
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
        anyhow::bail!("Unexpected stop. Should not be called and always be locked by another thermostat action")
    }

    fn controls_resource(&self) -> Option<Resource> {
        Some(self.heating_zone.resource())
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
            .map(|v| v == DegreeCelsius(7.0))
    }

    async fn is_user_controlled(&self) -> Result<bool> {
        self.heating_zone.user_controlled().current().await
    }

    async fn start(&self) -> Result<()> {
        Command::SetHeating {
            device: self.heating_zone.thermostat(),
            target_state: api::command::HeatingTargetState::Heat {
                temperature: DegreeCelsius(7.0),
            },
        }
        .execute()
        .await
    }

    async fn stop(&self) -> Result<()> {
        anyhow::bail!("Unexpected stop. Should not be called and always be locked by another thermostat action")
    }

    fn controls_resource(&self) -> Option<Resource> {
        Some(self.heating_zone.resource())
    }
}
