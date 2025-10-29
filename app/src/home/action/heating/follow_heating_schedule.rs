use crate::{
    core::{
        HomeApi,
        time::{DateTime, Duration},
        timeseries::{DataPoint, interpolate::algo::linear_dp},
    },
    home::{
        action::{Rule, RuleResult},
        command::Command,
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

        let target_state = self.zone.heating_state(&self.mode);
        let mut commands: Vec<Command> = self
            .zone
            .thermostats()
            .iter()
            .map(|thermostat| Command::SetHeating {
                target_state: target_state.clone(),
                device: thermostat.clone(),
            })
            .collect();

        //Hold thermostat ambient temperature in ventilation mode
        if active_mode == HeatingMode::Ventilation {
            commands.extend(self.hold_external_temperature(api).await?);
        }

        //Slowly move towards real temperature to avoid spikes after ventilation
        if active_mode == HeatingMode::PostVentilation {
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

        Ok(RuleResult::Execute(commands))
    }
}

impl FollowHeatingSchedule {
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

            if let Some(interpolated_temp) = linear_dp(t!(now), &prev, &next) {
                commands.push(Command::SetThermostatAmbientTemperature {
                    device: thermostat.clone(),
                    temperature: interpolated_temp,
                });
            }
        }

        Ok(commands)
    }
}
