mod auto_temp_increase;
mod ventilation_in_progress;
mod wait_for_sleeping;
mod wait_for_ventilation;

use std::fmt::Display;

use api::{
    command::{HeatingTargetState, SetHeating, Thermostat},
    state::{ExternalAutoControl, SetPoint},
};
use support::{ext::ToOk, t, time::DateTime, unit::DegreeCelsius, DataPoint};

pub use auto_temp_increase::NoHeatingDuringAutomaticTemperatureIncrease;
pub use ventilation_in_progress::NoHeatingDuringVentilation;
pub use wait_for_sleeping::ExtendHeatingUntilSleeping;
pub use wait_for_ventilation::DeferHeatingUntilVentilationDone;

use super::{CommandAccess, DataPointAccess};

#[derive(Debug, Clone)]
pub enum HeatingZone {
    LivingRoom,
    Bedroom,
    Kitchen,
    RoomOfRequirements,
    Bathroom,
}

impl HeatingZone {
    pub fn thermostat(&self) -> Thermostat {
        match self {
            HeatingZone::LivingRoom => Thermostat::LivingRoom,
            HeatingZone::Bedroom => Thermostat::Bedroom,
            HeatingZone::Kitchen => Thermostat::Kitchen,
            HeatingZone::RoomOfRequirements => Thermostat::RoomOfRequirements,
            HeatingZone::Bathroom => Thermostat::Bathroom,
        }
    }

    pub fn current_set_point(&self) -> SetPoint {
        match self {
            HeatingZone::LivingRoom => SetPoint::LivingRoom,
            HeatingZone::Bedroom => SetPoint::Bedroom,
            HeatingZone::Kitchen => SetPoint::Kitchen,
            HeatingZone::RoomOfRequirements => SetPoint::RoomOfRequirements,
            HeatingZone::Bathroom => SetPoint::Bathroom,
        }
    }

    pub fn auto_mode(&self) -> ExternalAutoControl {
        match self {
            HeatingZone::LivingRoom => ExternalAutoControl::LivingRoomThermostat,
            HeatingZone::Bedroom => ExternalAutoControl::BedroomThermostat,
            HeatingZone::Kitchen => ExternalAutoControl::KitchenThermostat,
            HeatingZone::RoomOfRequirements => ExternalAutoControl::RoomOfRequirementsThermostat,
            HeatingZone::Bathroom => ExternalAutoControl::BathroomThermostat,
        }
    }

    async fn is_manual_heating_to<T>(
        &self,
        api: &T,
        temperature: DegreeCelsius,
    ) -> anyhow::Result<DataPoint<bool>>
    where
        T: DataPointAccess<SetPoint> + DataPointAccess<ExternalAutoControl>,
    {
        let (set_point, auto_mode) = (self.current_set_point(), self.auto_mode());

        let (set_point, auto_mode) = tokio::try_join!(
            api.current_data_point(set_point),
            api.current_data_point(auto_mode)
        )?;

        Ok(DataPoint {
            value: set_point.value == temperature && !auto_mode.value,
            timestamp: std::cmp::max(set_point.timestamp, auto_mode.timestamp),
        })
    }

    async fn manual_heating_already_triggrered<T>(
        &self,
        api: &T,
        target_temperature: DegreeCelsius,
        since: DateTime,
    ) -> anyhow::Result<DataPoint<bool>>
    where
        T: CommandAccess<Thermostat>,
    {
        let commands = api.get_all_commands(self.thermostat(), since).await?;

        let trigger = commands.into_iter().find(|c| match c.command {
            SetHeating {
                target_state: HeatingTargetState::Heat { temperature, .. },
                ..
            } => temperature == target_temperature,
            _ => false,
        });

        if let Some(trigger) = trigger {
            DataPoint::new(true, trigger.created)
        } else {
            DataPoint::new(false, t!(now))
        }
        .to_ok()
    }
}

impl Display for HeatingZone {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HeatingZone::LivingRoom => write!(f, "LivingRoom"),
            HeatingZone::Bedroom => write!(f, "Bedroom"),
            HeatingZone::Kitchen => write!(f, "Kitchen"),
            HeatingZone::RoomOfRequirements => write!(f, "RoomOfRequirements"),
            HeatingZone::Bathroom => write!(f, "Bathroom"),
        }
    }
}
