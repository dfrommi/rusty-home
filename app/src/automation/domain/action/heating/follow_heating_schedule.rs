use crate::{
    automation::HeatingZone,
    command::{Command, HeatingTargetState},
    core::{
        time::{DateTime, Duration},
        timeseries::{
            DataPoint,
            interpolate::{Interpolator, LinearInterpolator},
        },
        unit::{DegreeCelsius, Percent},
    },
    home_state::{HeatingMode, TargetHeatingMode, Temperature},
    t,
};
use r#macro::Id;

use super::{Rule, RuleEvaluationContext, RuleResult};

#[derive(Debug, Clone, Id)]
pub struct FollowHeatingSchedule {
    zone: HeatingZone,
}

impl FollowHeatingSchedule {
    pub fn new(zone: HeatingZone) -> Self {
        Self { zone }
    }
}

impl Rule for FollowHeatingSchedule {
    fn evaluate(&self, ctx: &RuleEvaluationContext) -> anyhow::Result<RuleResult> {
        let active_mode_item = match self.zone {
            HeatingZone::RoomOfRequirements => TargetHeatingMode::RoomOfRequirements,
            HeatingZone::LivingRoom => TargetHeatingMode::LivingRoom,
            HeatingZone::Bedroom => TargetHeatingMode::Bedroom,
            HeatingZone::Kitchen => TargetHeatingMode::Kitchen,
            HeatingZone::Bathroom => TargetHeatingMode::Bathroom,
        };
        let active_mode_dp = ctx.current_dp(active_mode_item)?;

        let active_mode = active_mode_dp.value;
        let mode_active_since = active_mode_dp.timestamp;

        let mut commands: Vec<Command> = vec![];

        let target_temperature = self.zone.setpoint_for_mode(&active_mode);

        match active_mode {
            _ if self.zone == HeatingZone::RoomOfRequirements => {
                let current_temp = ctx.current(self.zone.inside_temperature())?;
                let max_opened = match active_mode {
                    HeatingMode::Ventilation => Percent(0.0),
                    HeatingMode::PostVentilation => Percent(25.0),
                    _ => Percent(80.0),
                };
                commands.extend(self.valve_open_position_command(target_temperature, current_temp, max_opened));
            }
            HeatingMode::Ventilation => {
                commands.extend(self.heating_state_commands(HeatingTargetState::WindowOpen));
                //Hold thermostat ambient temperature in ventilation mode
                commands.extend(self.hold_external_temperature(ctx)?);
            }
            HeatingMode::PostVentilation => {
                commands.extend(self.heat_to(target_temperature, ctx)?);

                if mode_active_since.elapsed() < t!(5 minutes) {
                    commands.extend(self.hold_external_temperature(ctx)?);
                } else {
                    commands.extend(self.move_towards_real_temperature(
                        mode_active_since,
                        TargetHeatingMode::post_ventilation_duration(),
                        ctx,
                    )?);
                }
            }

            _ => {
                commands.extend(self.heat_to(target_temperature, ctx)?);
            }
        }

        Ok(if let HeatingMode::Manual(_, trigger_id) = active_mode {
            RuleResult::ExecuteTrigger(commands, trigger_id)
        } else {
            RuleResult::Execute(commands)
        })
    }
}

impl FollowHeatingSchedule {
    //stupid heuristic. To be replaced with PID
    fn valve_open_position_command(
        &self,
        target_temperature: DegreeCelsius,
        current_temperature: DegreeCelsius,
        max_opened: Percent,
    ) -> Vec<Command> {
        let temp_diff = (target_temperature - current_temperature).0;
        //0.2 degree -> 20% opening
        let mut opening = (temp_diff / 0.2 * 20.0).clamp(0.0, max_opened.0).round();
        if opening < 10.0 {
            opening = 0.0; //deadzone
        }

        let opening_position = Percent(opening);

        tracing::trace!(
            "Setting valve opening position in zone {:?} to {:?} (target: {:?}, current: {:?})",
            self.zone,
            opening_position,
            target_temperature,
            current_temperature
        );

        self.zone
            .thermostats()
            .iter()
            .map(|thermostat| Command::SetThermostatValveOpeningPosition {
                device: *thermostat,
                value: opening_position,
            })
            .collect()
    }

    fn heat_to(&self, set_point: DegreeCelsius, ctx: &RuleEvaluationContext) -> anyhow::Result<Vec<Command>> {
        let mut commands: Vec<Command> = vec![];
        for thermostat in self.zone.thermostats() {
            let current_setpoint = ctx.current(thermostat.set_point())?;
            let is_increasing = set_point > current_setpoint;

            commands.push(Command::SetHeating {
                device: thermostat,
                target_state: HeatingTargetState::Heat {
                    temperature: set_point,
                    low_priority: is_increasing,
                },
            });
        }

        Ok(commands)
    }

    fn heating_state_commands(&self, target_state: HeatingTargetState) -> Vec<Command> {
        self.zone
            .thermostats()
            .iter()
            .map(|thermostat| Command::SetHeating {
                device: *thermostat,
                target_state: target_state.clone(),
            })
            .collect()
    }

    fn hold_external_temperature(&self, ctx: &RuleEvaluationContext) -> anyhow::Result<Vec<Command>> {
        let mut commands = vec![];
        for thermostat in self.zone.thermostats() {
            let thermostat_external_temp = ctx.current(Temperature::ThermostatExternal(thermostat))?;
            commands.push(Command::SetThermostatAmbientTemperature {
                device: thermostat,
                temperature: thermostat_external_temp,
            });
        }

        Ok(commands)
    }

    fn move_towards_real_temperature(
        &self,
        mode_start_time: DateTime,
        total_duration: Duration,
        ctx: &RuleEvaluationContext,
    ) -> anyhow::Result<Vec<Command>> {
        let mut commands = vec![];
        let room_temp = ctx.current(self.zone.inside_temperature())?;

        for thermostat in self.zone.thermostats() {
            let thermostat_external_temp = ctx.current(Temperature::ThermostatExternal(thermostat))?;

            if room_temp >= thermostat_external_temp {
                continue;
            }

            //reperated interpolation moves the value linearly towards room temp. Timestamp now is
            //therefore fine and on the line of linear interpolation
            let prev = DataPoint::new(thermostat_external_temp, t!(now));
            let next = DataPoint::new(room_temp, mode_start_time + total_duration.clone());

            let interpolated_temp = LinearInterpolator.interpolate(t!(now), &prev, &next)?;
            commands.push(Command::SetThermostatAmbientTemperature {
                device: thermostat,
                temperature: interpolated_temp,
            });
        }

        Ok(commands)
    }
}
