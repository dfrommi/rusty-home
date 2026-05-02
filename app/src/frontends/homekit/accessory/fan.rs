use crate::{
    core::unit::{FanAirflow, FanSpeed},
    frontends::homekit::{HomekitCharacteristic, HomekitEvent, HomekitService, HomekitTarget, HomekitTargetConfig},
    home_state::{FanActivity, HomeStateValue},
    trigger::UserTrigger,
};

const DEHUMIDIFIER_SPEEDS: [FanSpeed; 3] = [FanSpeed::Low, FanSpeed::Medium, FanSpeed::High];
const MIN_STEP: f64 = 100.0 / DEHUMIDIFIER_SPEEDS.len() as f64;

#[derive(Clone)]
struct FanStatus {
    airflow: FanAirflow,
    last_speed: FanSpeed,
}

impl FanStatus {
    fn new() -> Self {
        Self {
            airflow: FanAirflow::Off,
            last_speed: default_speed(),
        }
    }

    fn apply_state(&mut self, airflow: FanAirflow) {
        let airflow = normalize_airflow(&airflow);

        match &airflow {
            FanAirflow::Forward(speed) => {
                self.last_speed = speed.clone();
            }
            FanAirflow::Off => {}
        }

        self.airflow = airflow;
    }

    fn is_active(&self) -> bool {
        !matches!(self.airflow, FanAirflow::Off)
    }

    fn current_speed(&self) -> FanSpeed {
        match &self.airflow {
            FanAirflow::Forward(speed) => speed.clone(),
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
            status: FanStatus::new(),
        }
    }

    pub fn get_all_targets(&self) -> Vec<HomekitTargetConfig> {
        vec![
            self.target(HomekitCharacteristic::Active).into_config(),
            self.target(HomekitCharacteristic::RotationSpeed)
                .with_config(serde_json::json!({ "minStep": MIN_STEP })),
        ]
    }

    pub fn export_state(&mut self, state: &HomeStateValue) -> Vec<HomekitEvent> {
        match state {
            HomeStateValue::FanActivity(activity, airflow) if *activity == self.activity => {
                self.status.apply_state(airflow.clone());

                let speed_percent = airflow_to_percent(&self.status.airflow());
                let active = self.status.is_active();

                vec![
                    self.event(HomekitCharacteristic::Active, serde_json::json!(if active { 1 } else { 0 })),
                    self.event(HomekitCharacteristic::RotationSpeed, serde_json::json!(speed_percent)),
                ]
            }
            _ => Vec::new(),
        }
    }

    pub fn process_trigger(&mut self, trigger: &HomekitEvent) -> Option<UserTrigger> {
        if trigger.target == self.target(HomekitCharacteristic::Active) {
            if let Some(is_on) = value_to_bool(&trigger.value) {
                let new_airflow = if is_on {
                    if self.status.is_active() {
                        self.status.airflow()
                    } else {
                        let speed = self.status.current_speed();
                        FanAirflow::Forward(speed)
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
                    FanAirflow::Forward(percent_to_speed(percent))
                };

                return self.command_with_state(new_airflow);
            }

            tracing::warn!("Fan {} received invalid RotationSpeed payload: {}", self.name, trigger.value);
            return None;
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

    fn command_with_state(&mut self, airflow: FanAirflow) -> Option<UserTrigger> {
        let airflow = normalize_airflow(&airflow);

        if airflow == self.status.airflow() {
            return None;
        }

        self.status.apply_state(airflow.clone());

        Some(UserTrigger::FanSpeed {
            fan: self.activity,
            airflow,
        })
    }
}

fn default_speed() -> FanSpeed {
    DEHUMIDIFIER_SPEEDS.first().cloned().unwrap_or(FanSpeed::Low)
}

fn normalize_airflow(airflow: &FanAirflow) -> FanAirflow {
    match airflow {
        FanAirflow::Off => FanAirflow::Off,
        FanAirflow::Forward(speed) => FanAirflow::Forward(normalize_speed(speed)),
    }
}

fn normalize_speed(speed: &FanSpeed) -> FanSpeed {
    if DEHUMIDIFIER_SPEEDS.iter().any(|candidate| candidate == speed) {
        return speed.clone();
    }

    let target = speed_rank(speed);
    DEHUMIDIFIER_SPEEDS
        .iter()
        .min_by_key(|candidate| (speed_rank(candidate) - target).abs())
        .cloned()
        .unwrap_or_else(default_speed)
}

fn airflow_to_percent(airflow: &FanAirflow) -> f64 {
    match airflow {
        FanAirflow::Off => 0.0,
        FanAirflow::Forward(speed) => speed_to_percent(speed),
    }
}

fn speed_to_percent(speed: &FanSpeed) -> f64 {
    let speed = normalize_speed(speed);
    let index = DEHUMIDIFIER_SPEEDS
        .iter()
        .position(|candidate| candidate == &speed)
        .unwrap_or(0);

    (index as f64 + 1.0) * MIN_STEP
}

fn percent_to_speed(percent: f64) -> FanSpeed {
    let ratio = (percent / 100.0).clamp(0.0, 1.0);
    let raw_index = (ratio * DEHUMIDIFIER_SPEEDS.len() as f64).ceil() as usize;
    let index = raw_index.saturating_sub(1).min(DEHUMIDIFIER_SPEEDS.len() - 1);

    DEHUMIDIFIER_SPEEDS[index].clone()
}

fn speed_rank(speed: &FanSpeed) -> i32 {
    match speed {
        FanSpeed::Low => 1,
        FanSpeed::Medium => 2,
        FanSpeed::High => 3,
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
