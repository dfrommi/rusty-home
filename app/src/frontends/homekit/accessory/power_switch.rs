use super::{Accessory, HomekitCommand};
use crate::home_state::{HomeStateValue, PowerAvailable};
use crate::trigger::{OnOffDevice, UserTrigger};
use crate::{
    command::PowerToggle,
    frontends::homekit::{HomekitCharacteristic, HomekitEvent, HomekitService, HomekitTarget, HomekitTargetConfig},
};

pub struct PowerSwitch {
    name: &'static str,
    power_toggle: PowerToggle,
}

impl PowerSwitch {
    pub fn new(name: &'static str, power_toggle: PowerToggle) -> Self {
        Self { name, power_toggle }
    }
}

impl Accessory for PowerSwitch {
    fn get_all_targets(&self) -> Vec<HomekitTargetConfig> {
        vec![HomekitTarget::new(self.name.to_string(), HomekitService::Switch, HomekitCharacteristic::On).into_config()]
    }

    fn export_state(&mut self, state: &HomeStateValue) -> Vec<HomekitEvent> {
        let powered_item = match self.power_toggle {
            PowerToggle::Dehumidifier => PowerAvailable::Dehumidifier,
            PowerToggle::InfraredHeater => PowerAvailable::InfraredHeater,
            PowerToggle::LivingRoomTvAmbilight => PowerAvailable::LivingRoomTvAmbilight,
        };

        match state {
            HomeStateValue::PowerAvailable(powered, is_on) if powered == &powered_item => vec![HomekitEvent {
                target: HomekitTarget::new(self.name.to_string(), HomekitService::Switch, HomekitCharacteristic::On),
                value: serde_json::json!(is_on),
            }],
            _ => Vec::new(),
        }
    }

    fn process_trigger(&mut self, trigger: &HomekitEvent) -> Option<HomekitCommand> {
        if trigger.target
            == HomekitTarget::new(self.name.to_string(), HomekitService::Switch, HomekitCharacteristic::On)
            && let Some(is_on) = trigger.value.as_bool()
        {
            let on_off_device = match &self.power_toggle {
                PowerToggle::Dehumidifier => OnOffDevice::Dehumidifier,
                PowerToggle::InfraredHeater => OnOffDevice::InfraredHeater,
                PowerToggle::LivingRoomTvAmbilight => OnOffDevice::LivingRoomTvAmbilight,
            };
            return Some(HomekitCommand::immediate(UserTrigger::DevicePower {
                device: on_off_device,
                on: is_on,
            }));
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frontends::homekit::HomekitTarget;

    fn switch() -> PowerSwitch {
        PowerSwitch::new("Ambilight Wohnzimmer", PowerToggle::LivingRoomTvAmbilight)
    }

    #[test]
    fn exports_reported_ambilight_state_to_homekit() {
        let mut switch = switch();
        let events = switch.export_state(&HomeStateValue::PowerAvailable(PowerAvailable::LivingRoomTvAmbilight, true));

        assert_eq!(events.len(), 1);
        assert_eq!(
            events[0].target,
            HomekitTarget::new(
                "Ambilight Wohnzimmer".to_string(),
                HomekitService::Switch,
                HomekitCharacteristic::On,
            )
        );
        assert_eq!(events[0].value, serde_json::json!(true));
    }

    #[test]
    fn maps_homekit_switch_off_to_ambilight_power_trigger() {
        let mut switch = switch();
        let target = HomekitTarget::new(
            "Ambilight Wohnzimmer".to_string(),
            HomekitService::Switch,
            HomekitCharacteristic::On,
        );
        let command = switch
            .process_trigger(&HomekitEvent {
                target,
                value: serde_json::json!(false),
            })
            .expect("HomeKit switch command");

        assert!(matches!(
            command.trigger,
            UserTrigger::DevicePower {
                device: OnOffDevice::LivingRoomTvAmbilight,
                on: false,
            }
        ));
    }
}
