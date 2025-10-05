use crate::{
    home::{
        action::{Rule, RuleResult},
        command::Command,
        common::HeatingZone,
        state::{HeatingMode, ScheduledHeatingMode},
    },
    port::DataPointAccess,
};
use r#macro::Id;

#[derive(Debug, Clone, Id)]
pub struct FollowHeatingSchedule {
    zone: HeatingZone,
    mode: HeatingMode,
}

impl FollowHeatingSchedule {
    pub fn new(zone: HeatingZone, mode: HeatingMode) -> Self {
        Self { zone, mode }
    }
}

impl Rule for FollowHeatingSchedule {
    async fn evaluate(&self, api: &crate::core::HomeApi) -> anyhow::Result<RuleResult> {
        let active_mode = match self.zone {
            HeatingZone::RoomOfRequirements => ScheduledHeatingMode::RoomOfRequirements,
            HeatingZone::LivingRoom => ScheduledHeatingMode::LivingRoom,
            HeatingZone::Bedroom => ScheduledHeatingMode::Bedroom,
            HeatingZone::Kitchen => ScheduledHeatingMode::Kitchen,
            HeatingZone::Bathroom => ScheduledHeatingMode::Bathroom,
        }
        .current(api)
        .await?;

        if active_mode != self.mode {
            return Ok(RuleResult::Skip);
        }

        let target_state = self.zone.heating_state(&self.mode);
        let commands = self
            .zone
            .thermostats()
            .iter()
            .map(|thermostat| Command::SetHeating {
                target_state: target_state.clone(),
                device: thermostat.clone(),
            })
            .collect();

        //TODO PostVentilation and Sleep trigger once and keep running

        Ok(RuleResult::Execute(commands))
    }
}
