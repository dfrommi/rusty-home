use crate::home_state::{HomeStateValue, PowerAvailable};
use crate::{
    command::PowerToggle,
    frontends::homekit::{
        HomekitCharacteristic, HomekitCommand, HomekitEvent, HomekitService, HomekitTarget, HomekitTargetConfig,
    },
};

pub struct PowerSwitch {
    name: &'static str,
    power_toggle: PowerToggle,
}

impl PowerSwitch {
    pub fn new(name: &'static str, power_toggle: PowerToggle) -> Self {
        Self { name, power_toggle }
    }

    pub fn get_all_targets(&self) -> Vec<HomekitTargetConfig> {
        vec![HomekitTarget::new(self.name.to_string(), HomekitService::Switch, HomekitCharacteristic::On).into_config()]
    }

    pub fn export_state(&self, state: &HomeStateValue) -> Vec<HomekitEvent> {
        let powered_item = match self.power_toggle {
            PowerToggle::Dehumidifier => PowerAvailable::Dehumidifier,
            PowerToggle::InfraredHeater => PowerAvailable::InfraredHeater,
            PowerToggle::LivingRoomNotificationLight => PowerAvailable::LivingRoomNotificationLight,
        };

        match state {
            HomeStateValue::PowerAvailable(powered, is_on) if powered == &powered_item => vec![HomekitEvent {
                target: HomekitTarget::new(self.name.to_string(), HomekitService::Switch, HomekitCharacteristic::On),
                value: serde_json::json!(is_on),
            }],
            _ => Vec::new(),
        }
    }

    pub fn process_trigger(&self, trigger: &HomekitEvent) -> Option<HomekitCommand> {
        if trigger.target
            == HomekitTarget::new(self.name.to_string(), HomekitService::Switch, HomekitCharacteristic::On)
            && let Some(is_on) = trigger.value.as_bool()
        {
            return match self.power_toggle {
                PowerToggle::Dehumidifier => Some(HomekitCommand::DehumidifierPower(is_on)),
                PowerToggle::InfraredHeater => Some(HomekitCommand::InfraredHeaterPower(is_on)),
                PowerToggle::LivingRoomNotificationLight => {
                    tracing::error!("LivingRoomNotificationLight power toggle is not implemented in Homekit adapter");
                    None
                }
            };
        }

        None
    }
}
