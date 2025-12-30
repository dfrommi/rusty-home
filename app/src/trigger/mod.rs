mod adapter;
mod domain;
mod service;

use std::sync::Arc;

pub use domain::*;
use infrastructure::{EventBus, EventListener};
use sqlx::PgPool;

use crate::{
    core::time::{DateTime, DateTimeRange},
    t,
    trigger::{adapter::db::TriggerRepository, service::TriggerService},
};

#[derive(Debug, Clone)]
pub enum TriggerEvent {
    TriggerAdded,
}

#[derive(Debug, Clone)]
pub struct UserTriggerExecution {
    pub id: UserTriggerId,
    pub trigger: UserTrigger,
    pub timestamp: DateTime,
    pub active_until: Option<DateTime>,
    pub correlation_id: Option<String>,
}

impl UserTriggerExecution {
    pub fn target(&self) -> UserTriggerTarget {
        self.trigger.target()
    }

    pub fn is_active(&self) -> bool {
        let now = t!(now);
        match self.active_until {
            Some(active_until) => self.timestamp >= now && now < active_until,
            None => self.timestamp >= now,
        }
    }
}

pub struct TriggerModule {
    service: Arc<TriggerService>,
    event_bus: EventBus<TriggerEvent>,
}

#[derive(Clone)]
pub struct TriggerClient {
    service: Arc<TriggerService>,
}

impl TriggerClient {
    pub async fn add_trigger(&self, trigger: UserTrigger) -> anyhow::Result<()> {
        self.service.add_trigger(trigger).await
    }

    pub async fn get_all_active_triggers(&self) -> anyhow::Result<Vec<UserTriggerExecution>> {
        self.service.get_all_active_triggers().await
    }

    pub async fn get_all_triggers_active_anytime_in_range(
        &self,
        range: DateTimeRange,
    ) -> anyhow::Result<Vec<UserTriggerExecution>> {
        self.service.get_all_triggers_active_anytime_in_range(range).await
    }

    pub async fn disable_triggers_before_except(
        &self,
        before: DateTime,
        excluded_ids: &[UserTriggerId],
    ) -> anyhow::Result<u64> {
        self.service.disable_triggers_before_except(before, excluded_ids).await
    }
}

impl TriggerModule {
    pub fn new(pool: PgPool) -> Self {
        let repo = TriggerRepository::new(pool);
        let event_bus = EventBus::new(64);
        let service = Arc::new(TriggerService::new(repo, event_bus.emitter()));

        Self { service, event_bus }
    }

    pub fn client(&self) -> TriggerClient {
        TriggerClient {
            service: self.service.clone(),
        }
    }

    pub fn subscribe(&self) -> EventListener<TriggerEvent> {
        self.event_bus.subscribe()
    }
}
