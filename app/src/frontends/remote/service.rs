use crate::trigger::{RemoteTrigger, TriggerClient, UserTrigger};

pub struct RemoteService {
    trigger_client: TriggerClient,
}

impl RemoteService {
    pub fn new(trigger_client: TriggerClient) -> Self {
        Self { trigger_client }
    }

    pub fn handle_remote_trigger(&self, trigger: RemoteTrigger) {
        tracing::info!("Received remote trigger: {:?}", trigger);

        if let Err(e) = self.trigger_client.add_trigger(UserTrigger::Remote(trigger.clone())) {
            tracing::error!("Failed to persist remote trigger {:?}: {:?}", trigger, e);
        }
    }
}
