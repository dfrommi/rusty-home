use crate::{
    automation::Radiator,
    core::unit::Percent,
    frontends::homekit::{
        HomekitCharacteristic, HomekitCommand, HomekitEvent, HomekitService, HomekitTarget, HomekitTargetConfig,
    },
    home_state::{HeatingDemand, HomeStateValue},
};

#[derive(Clone, Copy)]
struct HeatingDemandStatus {
    current: Percent,
    last_non_zero: Percent,
}

impl Default for HeatingDemandStatus {
    fn default() -> Self {
        Self {
            current: Percent(0.0),
            last_non_zero: Percent(50.0),
        }
    }
}

pub struct HeatingDemandAccessory {
    name: &'static str,
    radiator: Radiator,
    demand: HeatingDemand,
    status: HeatingDemandStatus,
}

impl HeatingDemandAccessory {
    pub fn new(name: &'static str, radiator: Radiator) -> Self {
        Self {
            name,
            radiator,
            demand: radiator.current_heating_demand(),
            status: HeatingDemandStatus::default(),
        }
    }

    pub fn get_all_targets(&self) -> Vec<HomekitTargetConfig> {
        vec![
            self.target(HomekitCharacteristic::On).into_config(),
            self.target(HomekitCharacteristic::Brightness)
                .with_config(serde_json::json!({ "minStep": 5 })),
        ]
    }

    pub fn export_state(&mut self, state: &HomeStateValue) -> Vec<HomekitEvent> {
        match state {
            HomeStateValue::HeatingDemand(demand, value) if *demand == self.demand => {
                self.status.current = *value;
                if value.0 > 0.0 {
                    self.status.last_non_zero = *value;
                }

                let active = value.0 > 0.0;

                vec![
                    self.event(HomekitCharacteristic::On, serde_json::json!(active)),
                    self.event(HomekitCharacteristic::Brightness, serde_json::json!(value.0)),
                ]
            }
            _ => Vec::new(),
        }
    }

    pub fn process_trigger(&mut self, trigger: &HomekitEvent) -> Option<HomekitCommand> {
        if trigger.target == self.target(HomekitCharacteristic::Brightness) {
            if let Some(percent) = value_to_f64(&trigger.value) {
                let demand = Percent(percent.clamp(0.0, 100.0)).clamp();
                return self.command_with_demand(demand);
            }

            tracing::warn!(
                "Heating demand {} received invalid Brightness payload: {}",
                self.name,
                trigger.value
            );
            return None;
        }

        if trigger.target == self.target(HomekitCharacteristic::On) {
            if let Some(active) = value_to_bool(&trigger.value) {
                let demand = if active {
                    if self.status.last_non_zero.0 <= 0.0 {
                        Percent(50.0)
                    } else {
                        self.status.last_non_zero
                    }
                } else {
                    Percent(0.0)
                };

                return self.command_with_demand(demand);
            }

            tracing::warn!("Heating demand {} received invalid On payload: {}", self.name, trigger.value);
        }

        None
    }

    fn target(&self, characteristic: HomekitCharacteristic) -> HomekitTarget {
        HomekitTarget::new(self.name.to_string(), HomekitService::Lightbulb, characteristic)
    }

    fn event(&self, characteristic: HomekitCharacteristic, value: serde_json::Value) -> HomekitEvent {
        HomekitEvent {
            target: self.target(characteristic),
            value,
        }
    }

    fn command_with_demand(&mut self, demand: Percent) -> Option<HomekitCommand> {
        if demand == self.status.current {
            return None;
        }

        self.status.current = demand;
        if demand.0 > 0.0 {
            self.status.last_non_zero = demand;
        }

        Some(self.command(demand))
    }

    fn command(&self, demand: Percent) -> HomekitCommand {
        match self.radiator {
            Radiator::LivingRoomBig => HomekitCommand::LivingRoomBigHeatingDemand(demand),
            Radiator::LivingRoomSmall => HomekitCommand::LivingRoomSmallHeatingDemand(demand),
            Radiator::Bedroom => HomekitCommand::BedroomHeatingDemand(demand),
            Radiator::Kitchen => HomekitCommand::KitchenHeatingDemand(demand),
            Radiator::RoomOfRequirements => HomekitCommand::RoomOfRequirementsHeatingDemand(demand),
            Radiator::Bathroom => HomekitCommand::BathroomHeatingDemand(demand),
        }
    }
}

fn value_to_bool(value: &serde_json::Value) -> Option<bool> {
    value
        .as_bool()
        .or_else(|| value.as_i64().map(|v| v != 0))
        .or_else(|| value.as_f64().map(|v| (v - 0.0).abs() > f64::EPSILON))
        .or_else(|| {
            value.as_str().and_then(|raw| match raw.to_lowercase().as_str() {
                "true" | "1" => Some(true),
                "false" | "0" => Some(false),
                _ => None,
            })
        })
}

fn value_to_f64(value: &serde_json::Value) -> Option<f64> {
    value
        .as_f64()
        .or_else(|| value.as_i64().map(|v| v as f64))
        .or_else(|| value.as_str().and_then(|raw| raw.parse::<f64>().ok()))
}
