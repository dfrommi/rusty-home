use crate::{
    core::{
        HomeApi,
        time::{DateTime, Duration},
        timeseries::{DataPoint, interpolate::algo::linear_dp},
        unit::{DegreeCelsius, Percent},
    },
    home::{
        action::{Rule, RuleResult},
        command::{Command, HeatingTargetState},
        common::HeatingZone,
        state::{HeatingMode, ScheduledHeatingMode, Temperature},
    },
    port::DataPointAccess,
    t,
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
        let active_mode_dp = match self.zone {
            HeatingZone::RoomOfRequirements => ScheduledHeatingMode::RoomOfRequirements,
            HeatingZone::LivingRoom => ScheduledHeatingMode::LivingRoom,
            HeatingZone::Bedroom => ScheduledHeatingMode::Bedroom,
            HeatingZone::Kitchen => ScheduledHeatingMode::Kitchen,
            HeatingZone::Bathroom => ScheduledHeatingMode::Bathroom,
        }
        .current_data_point(api)
        .await?;

        let active_mode = active_mode_dp.value;
        let mode_active_since = active_mode_dp.timestamp;

        if active_mode != self.mode {
            return Ok(RuleResult::Skip);
        }

        let mut commands: Vec<Command> = vec![];
        let default_temperature = self.zone.default_setpoint();

        let target_temperature = match self.mode {
            HeatingMode::Ventilation => DegreeCelsius(0.0),
            HeatingMode::PostVentilation => default_temperature,
            HeatingMode::EnergySaving => default_temperature,
            HeatingMode::Sleep if self.zone == HeatingZone::LivingRoom => default_temperature - DegreeCelsius(0.5),
            HeatingMode::Comfort if self.zone == HeatingZone::LivingRoom => default_temperature + DegreeCelsius(0.5),
            HeatingMode::Sleep if self.zone == HeatingZone::Bedroom => default_temperature - DegreeCelsius(0.5),
            HeatingMode::Sleep => default_temperature - DegreeCelsius(1.0),
            HeatingMode::Comfort => default_temperature + DegreeCelsius(1.0),
            HeatingMode::Away => default_temperature - DegreeCelsius(2.0),
        };

        match self.mode {
            _ if self.zone == HeatingZone::RoomOfRequirements => {
                let current_temp = self.zone.inside_temperature().current(api).await?;
                let max_opened = match self.mode {
                    HeatingMode::Ventilation => Percent(0.0),
                    HeatingMode::PostVentilation => Percent(25.0),
                    _ => Percent(80.0),
                };
                commands.extend(self.valve_open_position_command(target_temperature, current_temp, max_opened));
            }
            HeatingMode::Ventilation => {
                commands.extend(self.heating_state_commands(HeatingTargetState::WindowOpen));
                //Hold thermostat ambient temperature in ventilation mode
                commands.extend(self.hold_external_temperature(api).await?);
            }
            HeatingMode::PostVentilation => {
                commands.extend(self.heat_to(default_temperature, api).await?);

                if mode_active_since.elapsed() < t!(5 minutes) {
                    commands.extend(self.hold_external_temperature(api).await?);
                } else {
                    commands.extend(
                        self.move_towards_real_temperature(
                            mode_active_since,
                            ScheduledHeatingMode::post_ventilation_duration(),
                            api,
                        )
                        .await?,
                    );
                }
            }

            _ => {
                commands.extend(self.heat_to(target_temperature, api).await?);
            }
        }

        Ok(RuleResult::Execute(commands))
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
        //0.5 degree -> 20% opening
        let opening = (temp_diff * 2.0 * 20.0).clamp(0.0, max_opened.0).round();
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
                device: thermostat.clone(),
                value: opening_position,
            })
            .collect()
    }

    async fn heat_to(&self, set_point: DegreeCelsius, api: &HomeApi) -> anyhow::Result<Vec<Command>> {
        let mut commands: Vec<Command> = vec![];
        for thermostat in self.zone.thermostats() {
            let current_setpoint = thermostat.set_point().current(api).await?;
            let is_increasing = set_point > current_setpoint;

            commands.push(Command::SetHeating {
                device: thermostat.clone(),
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
                device: thermostat.clone(),
                target_state: target_state.clone(),
            })
            .collect()
    }

    async fn hold_external_temperature(&self, api: &HomeApi) -> anyhow::Result<Vec<Command>> {
        let mut commands = vec![];
        for thermostat in self.zone.thermostats() {
            let thermostat_external_temp = Temperature::ThermostatExternal(thermostat.clone()).current(api).await?;
            commands.push(Command::SetThermostatAmbientTemperature {
                device: thermostat.clone(),
                temperature: thermostat_external_temp,
            });
        }

        Ok(commands)
    }

    async fn move_towards_real_temperature(
        &self,
        mode_start_time: DateTime,
        total_duration: Duration,
        api: &HomeApi,
    ) -> anyhow::Result<Vec<Command>> {
        let mut commands = vec![];
        let room_temp = self.zone.inside_temperature().current(api).await?;

        for thermostat in self.zone.thermostats() {
            let thermostat_external_temp = Temperature::ThermostatExternal(thermostat.clone()).current(api).await?;

            if room_temp >= thermostat_external_temp {
                continue;
            }

            //reperated interpolation moves the value linearly towards room temp. Timestamp now is
            //therefore fine and on the line of linear interpolation
            let prev = DataPoint::new(thermostat_external_temp, t!(now));
            let next = DataPoint::new(room_temp, mode_start_time + total_duration.clone());

            let interpolated_temp = linear_dp(t!(now), &prev, &next)?;
            commands.push(Command::SetThermostatAmbientTemperature {
                device: thermostat.clone(),
                temperature: interpolated_temp,
            });
        }

        Ok(commands)
    }
}
