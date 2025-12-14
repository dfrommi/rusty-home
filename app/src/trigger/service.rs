use tokio::sync::broadcast;

use crate::{
    core::time::DateTime,
    t,
    trigger::{TriggerEvent, UserTrigger, UserTriggerExecution, UserTriggerId, adapter::db::TriggerRepository},
};

pub struct TriggerService {
    repo: TriggerRepository,
    event_tx: broadcast::Sender<TriggerEvent>,
}

impl TriggerService {
    pub fn new(repo: TriggerRepository) -> Self {
        Self {
            repo,
            event_tx: broadcast::channel(100).0,
        }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<TriggerEvent> {
        self.event_tx.subscribe()
    }

    pub async fn add_trigger(&self, trigger: UserTrigger) -> anyhow::Result<()> {
        match self.repo.add_trigger(trigger).await {
            Ok(_) => {
                self.event_tx
                    .send(TriggerEvent::TriggerAdded)
                    .map_err(|e| anyhow::anyhow!("Failed to send trigger added event: {:?}", e))?;
                Ok(())
            }
            Err(e) => Err(e),
        }
    }

    pub async fn get_all_active_triggers(&self) -> anyhow::Result<Vec<UserTriggerExecution>> {
        self.repo.get_all_active_triggers_since(t!(48 hours ago)).await
    }

    pub async fn disable_triggers_before_except(
        &self,
        before: DateTime,
        excluded_ids: &[UserTriggerId],
    ) -> anyhow::Result<u64> {
        self.repo.cancel_triggers_before_excluding(before, excluded_ids).await
    }
}
