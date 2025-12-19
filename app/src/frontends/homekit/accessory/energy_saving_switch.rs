use crate::{
    frontends::homekit::{
        HomekitCharacteristic, HomekitCommand, HomekitEvent, HomekitService, HomekitTarget, HomekitTargetConfig,
    },
    home_state::{EnergySaving, HomeStateValue},
};

pub struct EnergySavingSwitch {
    name: &'static str,
    target: EnergySaving,
}

impl EnergySavingSwitch {
    pub fn new(name: &'static str, target: EnergySaving) -> Self {
        Self { name, target }
    }

    pub fn get_all_targets(&self) -> Vec<HomekitTargetConfig> {
        vec![self.target_config()]
    }

    pub fn export_state(&self, state: &HomeStateValue) -> Vec<HomekitEvent> {
        match state {
            HomeStateValue::EnergySaving(id, enabled) if *id == self.target => vec![HomekitEvent {
                target: self.homekit_target(),
                value: serde_json::json!(!enabled),
            }],
            _ => Vec::new(),
        }
    }

    pub fn process_trigger(&self, trigger: &HomekitEvent) -> Option<HomekitCommand> {
        if trigger.target == self.homekit_target() {
            if let Some(is_on) = trigger.value.as_bool() {
                let energy_saving = !is_on;
                return Some(HomekitCommand::LivingRoomTvEnergySaving(energy_saving));
            }

            tracing::warn!("EnergySavingSwitch {} received invalid payload: {}", self.name, trigger.value);
        }

        None
    }

    fn homekit_target(&self) -> HomekitTarget {
        HomekitTarget::new(self.name.to_string(), HomekitService::Switch, HomekitCharacteristic::On)
    }

    fn target_config(&self) -> HomekitTargetConfig {
        self.homekit_target().into_config()
    }
}
