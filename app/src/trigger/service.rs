use infrastructure::EventEmitter;

use crate::{
    core::time::{DateTime, DateTimeRange},
    t,
    trigger::{TriggerEvent, UserTrigger, UserTriggerExecution, UserTriggerId, adapter::db::TriggerRepository},
};

pub struct TriggerService {
    repo: TriggerRepository,
    event_tx: EventEmitter<TriggerEvent>,
}

impl TriggerService {
    pub fn new(repo: TriggerRepository, event_tx: EventEmitter<TriggerEvent>) -> Self {
        Self { repo, event_tx }
    }

    pub async fn add_trigger(&self, trigger: UserTrigger) -> anyhow::Result<()> {
        self.repo.add_trigger(trigger).await?;
        self.event_tx.send(TriggerEvent::TriggerAdded);
        Ok(())
    }

    pub async fn get_all_triggers_active_anytime_in_range(
        &self,
        range: DateTimeRange,
    ) -> anyhow::Result<Vec<UserTriggerExecution>> {
        self.repo.get_all_triggers_active_anytime_in_range(range).await
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

    pub async fn set_triggers_active_from_if_unset(&self, trigger_ids: &[UserTriggerId]) -> anyhow::Result<u64> {
        self.repo.set_triggers_active_from_if_unset(trigger_ids).await
    }
}
