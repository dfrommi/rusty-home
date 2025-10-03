use std::fmt::Display;

use crate::{
    core::planner::{Action, ActionEvaluationResult},
    home::{
        command::Command,
        common::HeatingZone,
        state::{HeatingMode, ScheduledHeatingMode},
    },
    port::DataPointAccess,
};

#[derive(Debug, Clone)]
pub struct FollowHeatingSchedule {
    zone: HeatingZone,
    mode: HeatingMode,
}

impl FollowHeatingSchedule {
    pub fn new(zone: HeatingZone, mode: HeatingMode) -> Self {
        Self { zone, mode }
    }
}

impl Display for FollowHeatingSchedule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "FollowHeatingSchedule[{} - {}]", self.zone, self.mode)
    }
}

impl Action for FollowHeatingSchedule {
    async fn evaluate(&self, api: &crate::core::HomeApi) -> anyhow::Result<ActionEvaluationResult> {
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
            return Ok(ActionEvaluationResult::Skip);
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

        Ok(ActionEvaluationResult::ExecuteMulti(commands, super::action_source(self)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{home::common::HeatingZone, home::state::HeatingMode};

    #[test]
    fn display_includes_zone_and_mode() {
        let action = FollowHeatingSchedule::new(HeatingZone::LivingRoom, HeatingMode::EnergySaving);
        assert_eq!(action.to_string(), "FollowHeatingSchedule[LivingRoom - EnergySaving]");
    }
}
