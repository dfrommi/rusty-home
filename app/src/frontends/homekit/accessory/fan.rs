use crate::{
    core::unit::{FanAirflow, FanSpeed},
    frontends::homekit::{
        HomekitCharacteristic, HomekitCommand, HomekitEvent, HomekitService, HomekitTarget, HomekitTargetConfig,
    },
    home_state::{FanActivity, HomeStateValue},
};

const ALL_SPEEDS: [FanSpeed; 5] = [
    FanSpeed::Silent,
    FanSpeed::Low,
    FanSpeed::Medium,
    FanSpeed::High,
    FanSpeed::Turbo,
];
const DEHUMIDIFIER_SPEEDS: [FanSpeed; 3] = [FanSpeed::Low, FanSpeed::Medium, FanSpeed::High];

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

#[derive(Clone, Copy)]
struct FanConfig {
    supports_reverse: bool,
    speeds: &'static [FanSpeed],
}

impl FanConfig {
    fn for_activity(activity: FanActivity) -> Self {
        match activity {
            FanActivity::BedroomDehumidifier => Self {
                supports_reverse: false,
                speeds: &DEHUMIDIFIER_SPEEDS,
            },
            FanActivity::LivingRoomCeilingFan | FanActivity::BedroomCeilingFan => Self {
                supports_reverse: true,
                speeds: &ALL_SPEEDS,
            },
        }
    }

    fn min_step(&self) -> f64 {
        if self.speeds.is_empty() {
            100.0
        } else {
            100.0 / self.speeds.len() as f64
        }
    }

    fn default_speed(&self) -> FanSpeed {
        self.speeds.first().cloned().unwrap_or(FanSpeed::Low)
    }

    fn normalize_speed(&self, speed: &FanSpeed) -> FanSpeed {
        if self.speeds.iter().any(|candidate| candidate == speed) {
            return speed.clone();
        }

        let target = speed_rank(speed);
        self.speeds
            .iter()
            .min_by_key(|candidate| (speed_rank(candidate) - target).abs())
            .cloned()
            .unwrap_or_else(|| self.default_speed())
    }

    fn normalize_airflow(&self, airflow: &FanAirflow) -> FanAirflow {
        match airflow {
            FanAirflow::Off => FanAirflow::Off,
            FanAirflow::Forward(speed) => FanAirflow::Forward(self.normalize_speed(speed)),
            FanAirflow::Reverse(speed) => {
                let speed = self.normalize_speed(speed);
                if self.supports_reverse {
                    FanAirflow::Reverse(speed)
                } else {
                    FanAirflow::Forward(speed)
                }
            }
        }
    }

    fn airflow_to_percent(&self, airflow: &FanAirflow) -> f64 {
        match airflow {
            FanAirflow::Off => 0.0,
            FanAirflow::Forward(speed) | FanAirflow::Reverse(speed) => self.speed_to_percent(speed),
        }
    }

    fn speed_to_percent(&self, speed: &FanSpeed) -> f64 {
        let speed = self.normalize_speed(speed);
        let index = self
            .speeds
            .iter()
            .position(|candidate| candidate == &speed)
            .unwrap_or(0);

        (index as f64 + 1.0) * self.min_step()
    }

    fn percent_to_speed(&self, percent: f64) -> FanSpeed {
        if self.speeds.is_empty() {
            return FanSpeed::Low;
        }

        let ratio = (percent / 100.0).clamp(0.0, 1.0);
        let raw_index = (ratio * self.speeds.len() as f64).ceil() as usize;
        let index = raw_index.saturating_sub(1).min(self.speeds.len() - 1);

        self.speeds[index].clone()
    }
}

#[derive(Clone)]
struct FanStatus {
    airflow: FanAirflow,
    last_direction: FanDirection,
    last_speed: FanSpeed,
}

impl FanStatus {
    fn new(default_speed: FanSpeed) -> Self {
        Self {
            airflow: FanAirflow::Off,
            last_direction: FanDirection::Forward,
            last_speed: default_speed,
        }
    }

    fn apply_state(&mut self, airflow: FanAirflow, config: &FanConfig) {
        let airflow = config.normalize_airflow(&airflow);

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
    config: FanConfig,
    status: FanStatus,
}

impl Fan {
    pub fn new(name: &'static str, activity: FanActivity) -> Self {
        let config = FanConfig::for_activity(activity);

        Self {
            name,
            activity,
            config,
            status: FanStatus::new(config.default_speed()),
        }
    }

    pub fn get_all_targets(&self) -> Vec<HomekitTargetConfig> {
        let mut targets = vec![
            self.target(HomekitCharacteristic::Active).into_config(),
            self.target(HomekitCharacteristic::RotationSpeed)
                .with_config(serde_json::json!({ "minStep": self.config.min_step() })),
        ];

        if self.config.supports_reverse {
            targets.push(self.target(HomekitCharacteristic::RotationDirection).into_config());
        }

        targets
    }

    pub fn export_state(&mut self, state: &HomeStateValue) -> Vec<HomekitEvent> {
        match state {
            HomeStateValue::FanActivity(activity, airflow) if *activity == self.activity => {
                self.status.apply_state(airflow.clone(), &self.config);

                let direction = self.status.current_direction();
                let speed_percent = self.config.airflow_to_percent(&self.status.airflow());
                let active = self.status.is_active();

                let mut events = vec![
                    self.event(HomekitCharacteristic::Active, serde_json::json!(if active { 1 } else { 0 })),
                    self.event(HomekitCharacteristic::RotationSpeed, serde_json::json!(speed_percent)),
                ];

                if self.config.supports_reverse {
                    events.push(self.event(
                        HomekitCharacteristic::RotationDirection,
                        serde_json::json!(direction.characteristic_value()),
                    ));
                }

                events
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

            tracing::warn!("Fan {} received invalid Active payload: {}", self.name, trigger.value);
            return None;
        }

        if trigger.target == self.target(HomekitCharacteristic::RotationSpeed) {
            if let Some(percent) = value_to_f64(&trigger.value) {
                let percent = percent.clamp(0.0, 100.0);

                let new_airflow = if percent <= 0.0 {
                    FanAirflow::Off
                } else {
                    let speed = self.config.percent_to_speed(percent);
                    let direction = if self.config.supports_reverse {
                        self.status.current_direction()
                    } else {
                        FanDirection::Forward
                    };
                    direction.with_speed(speed)
                };

                return self.command_with_state(new_airflow);
            }

            tracing::warn!("Fan {} received invalid RotationSpeed payload: {}", self.name, trigger.value);
            return None;
        }

        if trigger.target == self.target(HomekitCharacteristic::RotationDirection) {
            if !self.config.supports_reverse {
                tracing::warn!(
                    "Fan {} received RotationDirection payload, but reverse is not supported",
                    self.name
                );
                return None;
            }

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
        let airflow = self.config.normalize_airflow(&airflow);

        if airflow == self.status.airflow() {
            return None;
        }

        self.status.apply_state(airflow.clone(), &self.config);

        match self.activity {
            FanActivity::LivingRoomCeilingFan => Some(HomekitCommand::LivingRoomCeilingFanSpeed(airflow)),
            FanActivity::BedroomCeilingFan => Some(HomekitCommand::BedroomCeilingFanSpeed(airflow)),
            FanActivity::BedroomDehumidifier => Some(HomekitCommand::BedroomDehumidifierFanSpeed(airflow)),
        }
    }
}

fn speed_rank(speed: &FanSpeed) -> i32 {
    match speed {
        FanSpeed::Silent => 0,
        FanSpeed::Low => 1,
        FanSpeed::Medium => 2,
        FanSpeed::High => 3,
        FanSpeed::Turbo => 4,
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
