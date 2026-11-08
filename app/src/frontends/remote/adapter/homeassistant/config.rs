use crate::trigger::RemoteTriggerTarget;

pub fn default_ha_remote_config() -> Vec<(&'static str, RemoteTriggerTarget)> {
    vec![("dimmer_bett_button", RemoteTriggerTarget::BedframeRemote)]
}
