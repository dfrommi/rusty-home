use crate::{
    adapter::homekit::{
        HomekitCharacteristic, HomekitCommand, HomekitEvent, HomekitHeatingState, HomekitService, HomekitTarget,
        HomekitTargetConfig,
    },
    core::unit::DegreeCelsius,
    home::{
        HeatingZone,
        state::{HeatingDemand, HomeStateValue, SetPoint, Temperature, UserControlled},
    },
};

#[derive(Default, Clone, Copy)]
struct ThermostatStatus {
    set_point: Option<DegreeCelsius>,
    user_controlled: Option<bool>,
    display_units_sent: bool,
}

pub struct Thermostat {
    name: &'static str,
    zone: HeatingZone,
    temperature: Temperature,
    set_point: SetPoint,
    user_controlled: UserControlled,
    heating_demand: HeatingDemand,
    status: ThermostatStatus,
}

impl Thermostat {
    pub fn new(name: &'static str, zone: HeatingZone) -> Self {
        let (temperature, set_point, user_controlled, heating_demand) = match zone {
            //TODO handle multiple radiators properly
            HeatingZone::LivingRoom => (
                Temperature::LivingRoom,
                SetPoint::LivingRoomBig,
                UserControlled::LivingRoomThermostatBig,
                HeatingDemand::LivingRoomBig,
            ),
            HeatingZone::Bedroom => (
                Temperature::Bedroom,
                SetPoint::Bedroom,
                UserControlled::BedroomThermostat,
                HeatingDemand::Bedroom,
            ),
            HeatingZone::Kitchen => (
                Temperature::Kitchen,
                SetPoint::Kitchen,
                UserControlled::KitchenThermostat,
                HeatingDemand::Kitchen,
            ),
            HeatingZone::RoomOfRequirements => (
                Temperature::RoomOfRequirements,
                SetPoint::RoomOfRequirements,
                UserControlled::RoomOfRequirementsThermostat,
                HeatingDemand::RoomOfRequirements,
            ),
            HeatingZone::Bathroom => (
                Temperature::BathroomShower,
                SetPoint::Bathroom,
                UserControlled::BathroomThermostat,
                HeatingDemand::Bathroom,
            ),
        };

        Self {
            name,
            zone,
            temperature,
            set_point,
            user_controlled,
            heating_demand,
            status: ThermostatStatus::default(),
        }
    }

    pub fn get_all_targets(&self) -> Vec<HomekitTargetConfig> {
        vec![
            self.target(HomekitCharacteristic::CurrentTemperature).into_config(),
            self.target(HomekitCharacteristic::TargetTemperature).into_config(),
            self.target(HomekitCharacteristic::CurrentHeatingCoolingState)
                .with_config(serde_json::json!({ "validValues": [0, 1] })),
            self.target(HomekitCharacteristic::TargetHeatingCoolingState)
                .with_config(serde_json::json!({ "validValues": [0, 1, 3] })),
            self.target(HomekitCharacteristic::TemperatureDisplayUnits)
                .into_config(),
        ]
    }

    pub fn export_state(&mut self, state: &HomeStateValue) -> Vec<HomekitEvent> {
        let mut events = Vec::new();

        if !self.status.display_units_sent {
            self.status.display_units_sent = true;
            //Celsius
            events.push(self.event(HomekitCharacteristic::TemperatureDisplayUnits, serde_json::json!(0)));
        }

        match state {
            HomeStateValue::Temperature(temperature, value) if *temperature == self.temperature => {
                events.push(self.event(HomekitCharacteristic::CurrentTemperature, serde_json::json!(value.0)));
            }
            HomeStateValue::SetPoint(set_point, value) if *set_point == self.set_point => {
                self.status.set_point = Some(*value);

                events.push(self.event(HomekitCharacteristic::TargetTemperature, serde_json::json!(value.0)));
                if let Some(event) = self.target_state_event() {
                    events.push(event);
                }
            }
            HomeStateValue::UserControlled(user_controlled, value) if *user_controlled == self.user_controlled => {
                self.status.user_controlled = Some(*value);

                if let Some(event) = self.target_state_event() {
                    events.push(event);
                }
            }
            HomeStateValue::HeatingDemand(demand, value) if *demand == self.heating_demand => {
                let state = if value.0 > 0.0 { 1 } else { 0 };
                events.push(self.event(HomekitCharacteristic::CurrentHeatingCoolingState, serde_json::json!(state)));
            }
            _ => {}
        }

        events
    }

    pub fn process_trigger(&self, trigger: &HomekitEvent) -> Option<HomekitCommand> {
        if trigger.target == self.target(HomekitCharacteristic::TargetTemperature) {
            let target_temp = trigger
                .value
                .as_f64()
                .or_else(|| trigger.value.as_str().and_then(|value| value.parse::<f64>().ok()));

            if let Some(target_temp) = target_temp {
                let temperature = DegreeCelsius(target_temp);
                return Some(self.zone_command(HomekitHeatingState::Heat(temperature)));
            }

            tracing::warn!(
                "Thermostat {} received invalid target temperature payload: {}",
                self.name,
                trigger.value
            );

            return None;
        }

        if trigger.target == self.target(HomekitCharacteristic::TargetHeatingCoolingState) {
            let state = trigger
                .value
                .as_i64()
                .or_else(|| trigger.value.as_str().and_then(|value| value.parse::<i64>().ok()));

            if let Some(state) = state {
                return match state {
                    0 => Some(self.zone_command(HomekitHeatingState::Off)),
                    1 => {
                        let temperature = self.status.set_point.unwrap_or_else(|| self.zone.default_setpoint());
                        Some(self.zone_command(HomekitHeatingState::Heat(temperature)))
                    }
                    3 => Some(self.zone_command(HomekitHeatingState::Auto)),
                    unsupported => {
                        tracing::warn!(
                            "Thermostat {} received unsupported TargetHeatingCoolingState value: {}",
                            self.name,
                            unsupported
                        );
                        None
                    }
                };
            }

            tracing::warn!(
                "Thermostat {} received invalid target heating state payload: {}",
                self.name,
                trigger.value
            );
        }

        None
    }

    fn event(&self, characteristic: HomekitCharacteristic, value: serde_json::Value) -> HomekitEvent {
        HomekitEvent {
            target: self.target(characteristic),
            value,
        }
    }

    fn target(&self, characteristic: HomekitCharacteristic) -> HomekitTarget {
        HomekitTarget::new(self.name.to_string(), HomekitService::Thermostat, characteristic)
    }

    fn zone_command(&self, heating_state: HomekitHeatingState) -> HomekitCommand {
        match self.zone {
            HeatingZone::LivingRoom => HomekitCommand::LivingRoomHeatingState(heating_state),
            HeatingZone::Bedroom => HomekitCommand::BedroomHeatingState(heating_state),
            HeatingZone::Kitchen => HomekitCommand::KitchenHeatingState(heating_state),
            HeatingZone::RoomOfRequirements => HomekitCommand::RoomOfRequirementsHeatingState(heating_state),
            HeatingZone::Bathroom => HomekitCommand::BathroomHeatingState(heating_state),
        }
    }

    fn target_state_event(&self) -> Option<HomekitEvent> {
        let set_point = self.status.set_point?;
        let user_controlled = self.status.user_controlled?;

        let value = if user_controlled {
            1 // Heat (manual override)
        } else if set_point > DegreeCelsius(0.0) {
            3 // Auto (schedule-driven heating)
        } else {
            0 // Off
        };

        Some(self.event(HomekitCharacteristic::TargetHeatingCoolingState, serde_json::json!(value)))
    }
}
