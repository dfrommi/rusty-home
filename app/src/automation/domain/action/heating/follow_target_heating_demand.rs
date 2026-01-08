use crate::{
    automation::HeatingZone,
    command::Command,
    home_state::{HeatingMode, TargetHeatingDemand, TargetHeatingMode},
};
use r#macro::Id;

use super::{Rule, RuleEvaluationContext, RuleResult};

#[derive(Debug, Clone, Id)]
pub struct FollowTargetHeatingDemand {
    zone: HeatingZone,
}

impl FollowTargetHeatingDemand {
    pub fn new(zone: HeatingZone) -> Self {
        Self { zone }
    }
}

impl Rule for FollowTargetHeatingDemand {
    fn evaluate(&self, ctx: &RuleEvaluationContext) -> anyhow::Result<RuleResult> {
        let thermostats = self.zone.thermostats();

        let commands = thermostats
            .into_iter()
            .map(|thermostat| {
                let demand = ctx.current(TargetHeatingDemand::ByRadiatorTemperature(thermostat))?;
                Ok(Command::SetThermostatValveOpeningPosition {
                    device: thermostat,
                    value: demand,
                })
            })
            .collect::<anyhow::Result<Vec<_>>>()?;

        //Not ideal but needed to keep the trigger going. Look for better solution with triggers
        let mode = ctx.current(match self.zone {
            HeatingZone::RoomOfRequirements => TargetHeatingMode::RoomOfRequirements,
            HeatingZone::LivingRoom => TargetHeatingMode::LivingRoom,
            HeatingZone::Bedroom => TargetHeatingMode::Bedroom,
            HeatingZone::Kitchen => TargetHeatingMode::Kitchen,
            HeatingZone::Bathroom => TargetHeatingMode::Bathroom,
        })?;

        if let HeatingMode::Manual(_, trigger_id) = mode {
            Ok(RuleResult::ExecuteTrigger(commands, trigger_id))
        } else {
            Ok(RuleResult::Execute(commands))
        }
    }
}
