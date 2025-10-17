use crate::{
    adapter::{
        homebridge::{HomekitCharacteristic, HomekitEvent, HomekitService, HomekitTarget, HomekitTargetConfig},
        homekit::HomekitCommand,
    },
    home::state::{FanActivity, FanAirflow, FanSpeed, HomeStateValue},
};

#[derive(Clone, Copy, Eq, PartialEq)]
enum FanDirection {
    Forward,
    Reverse,
}

impl FanDirection {
    fn from_airflow(airflow: &FanAirflow) -> Option<Self> {
        match airflow {
            FanAirflow::Forward(_) => Some(FanDirection::Forward),
            FanAirflow::Reverse(_) => Some(FanDirection::Reverse),
            FanAirflow::Off => None,
        }
    }

    fn characteristic_value(self) -> i64 {
        match self {
            FanDirection::Forward => 0,
            FanDirection::Reverse => 1,
        }
    }

    fn with_speed(self, speed: FanSpeed) -> FanAirflow {
        match self {
            FanDirection::Forward => FanAirflow::Forward(speed),
            FanDirection::Reverse => FanAirflow::Reverse(speed),
        }
    }

    fn try_from_value(value: &serde_json::Value) -> Option<Self> {
        value
            .as_i64()
            .or_else(|| value.as_f64().map(|v| v as i64))
            .or_else(|| value.as_str().and_then(|raw| raw.parse::<i64>().ok()))
            .and_then(|value| match value {
                0 => Some(FanDirection::Forward),
                1 => Some(FanDirection::Reverse),
                _ => None,
            })
    }
}

#[derive(Clone)]
struct FanStatus {
    airflow: FanAirflow,
    last_direction: FanDirection,
    last_speed: FanSpeed,
}

impl Default for FanStatus {
    fn default() -> Self {
        Self {
            airflow: FanAirflow::Off,
            last_direction: FanDirection::Forward,
            last_speed: FanSpeed::Silent,
        }
    }
}

impl FanStatus {
    fn apply_state(&mut self, airflow: FanAirflow) {
        match &airflow {
            FanAirflow::Forward(speed) => {
                self.last_direction = FanDirection::Forward;
                self.last_speed = speed.clone();
            }
            FanAirflow::Reverse(speed) => {
                self.last_direction = FanDirection::Reverse;
                self.last_speed = speed.clone();
            }
            FanAirflow::Off => {}
        }

        self.airflow = airflow;
    }

    fn is_active(&self) -> bool {
        !matches!(self.airflow, FanAirflow::Off)
    }

    fn current_direction(&self) -> FanDirection {
        FanDirection::from_airflow(&self.airflow).unwrap_or(self.last_direction)
    }

    fn current_speed(&self) -> FanSpeed {
        match &self.airflow {
            FanAirflow::Forward(speed) | FanAirflow::Reverse(speed) => speed.clone(),
            FanAirflow::Off => self.last_speed.clone(),
        }
    }

    fn airflow(&self) -> FanAirflow {
        self.airflow.clone()
    }
}

pub struct Fan {
    name: &'static str,
    activity: FanActivity,
    status: FanStatus,
}

impl Fan {
    pub fn new(name: &'static str, activity: FanActivity) -> Self {
        Self {
            name,
            activity,
            status: FanStatus::default(),
        }
    }

    pub fn get_all_targets(&self) -> Vec<HomekitTargetConfig> {
        vec![
            self.target(HomekitCharacteristic::Active).into_config(),
            self.target(HomekitCharacteristic::RotationSpeed)
                .with_config(serde_json::json!({ "minStep": 20 })),
            self.target(HomekitCharacteristic::RotationDirection).into_config(),
        ]
    }

    pub fn export_state(&mut self, state: &HomeStateValue) -> Vec<HomekitEvent> {
        match state {
            HomeStateValue::FanActivity(activity, airflow) if *activity == self.activity => {
                self.status.apply_state(airflow.clone());

                let direction = self.status.current_direction();
                let speed_percent = airflow_to_percent(&self.status.airflow());
                let active = self.status.is_active();

                vec![
                    self.event(HomekitCharacteristic::Active, serde_json::json!(if active { 1 } else { 0 })),
                    self.event(HomekitCharacteristic::RotationSpeed, serde_json::json!(speed_percent)),
                    self.event(
                        HomekitCharacteristic::RotationDirection,
                        serde_json::json!(direction.characteristic_value()),
                    ),
                ]
            }
            _ => Vec::new(),
        }
    }

    pub fn process_trigger(&mut self, trigger: &HomekitEvent) -> Option<HomekitCommand> {
        if trigger.target == self.target(HomekitCharacteristic::Active) {
            if let Some(is_on) = value_to_bool(&trigger.value) {
                let new_airflow = if is_on {
                    if self.status.is_active() {
                        self.status.airflow()
                    } else {
                        let direction = self.status.current_direction();
                        let speed = self.status.current_speed();
                        direction.with_speed(speed)
                    }
                } else {
                    FanAirflow::Off
                };

                return self.command_with_state(new_airflow);
            }

            tracing::warn!(
                "Fan {} received invalid Active payload: {}",
                self.name,
                trigger.value
            );
            return None;
        }

        if trigger.target == self.target(HomekitCharacteristic::RotationSpeed) {
            if let Some(percent) = value_to_f64(&trigger.value) {
                let percent = percent.clamp(0.0, 100.0);

                let new_airflow = if percent <= 0.0 {
                    FanAirflow::Off
                } else {
                    let speed = percent_to_speed(percent);
                    let direction = self.status.current_direction();
                    direction.with_speed(speed)
                };

                return self.command_with_state(new_airflow);
            }

            tracing::warn!(
                "Fan {} received invalid RotationSpeed payload: {}",
                self.name,
                trigger.value
            );
            return None;
        }

        if trigger.target == self.target(HomekitCharacteristic::RotationDirection) {
            if let Some(direction) = FanDirection::try_from_value(&trigger.value) {
                self.status.last_direction = direction;

                if self.status.is_active() {
                    let speed = self.status.current_speed();
                    let new_airflow = direction.with_speed(speed);
                    return self.command_with_state(new_airflow);
                }

                return None;
            }

            tracing::warn!(
                "Fan {} received invalid RotationDirection payload: {}",
                self.name,
                trigger.value
            );
        }

        None
    }

    fn target(&self, characteristic: HomekitCharacteristic) -> HomekitTarget {
        HomekitTarget::new(self.name.to_string(), HomekitService::Fanv2, characteristic)
    }

    fn event(&self, characteristic: HomekitCharacteristic, value: serde_json::Value) -> HomekitEvent {
        HomekitEvent {
            target: self.target(characteristic),
            value,
        }
    }

    fn command_with_state(&mut self, airflow: FanAirflow) -> Option<HomekitCommand> {
        if airflow == self.status.airflow() {
            return None;
        }

        self.status.apply_state(airflow.clone());

        Some(match self.activity {
            FanActivity::LivingRoomCeilingFan => HomekitCommand::LivingRoomCeilingFanSpeed(airflow),
            FanActivity::BedroomCeilingFan => HomekitCommand::BedroomCeilingFanSpeed(airflow),
        })
    }
}

fn airflow_to_percent(airflow: &FanAirflow) -> f64 {
    match airflow {
        FanAirflow::Off => 0.0,
        FanAirflow::Forward(speed) | FanAirflow::Reverse(speed) => speed_to_percent(speed),
    }
}

fn speed_to_percent(speed: &FanSpeed) -> f64 {
    match speed {
        FanSpeed::Silent => 20.0,
        FanSpeed::Low => 40.0,
        FanSpeed::Medium => 60.0,
        FanSpeed::High => 80.0,
        FanSpeed::Turbo => 100.0,
    }
}

fn percent_to_speed(percent: f64) -> FanSpeed {
    if percent <= 20.0 {
        FanSpeed::Silent
    } else if percent <= 40.0 {
        FanSpeed::Low
    } else if percent <= 60.0 {
        FanSpeed::Medium
    } else if percent <= 80.0 {
        FanSpeed::High
    } else {
        FanSpeed::Turbo
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
