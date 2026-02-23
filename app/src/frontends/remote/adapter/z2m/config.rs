use crate::trigger::RemoteTriggerTarget;

pub fn default_z2m_remote_config() -> Vec<(&'static str, RemoteTriggerTarget)> {
    vec![("bedroom/remote", RemoteTriggerTarget::BedroomDoorRemote)]
}
