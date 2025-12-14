mod adapter;
mod domain;
mod service;

use std::sync::Arc;

pub use domain::*;
use sqlx::PgPool;
use tokio::sync::broadcast;

use crate::{
    core::time::DateTime,
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
    pub correlation_id: Option<String>,
}

impl UserTriggerExecution {
    pub fn target(&self) -> UserTriggerTarget {
        self.trigger.target()
    }
}

pub struct TriggerRunner {
    service: Arc<TriggerService>,
}

#[derive(Clone)]
pub struct TriggerClient {
    service: Arc<TriggerService>,
}

impl TriggerClient {
    pub fn subscribe(&self) -> broadcast::Receiver<TriggerEvent> {
        self.service.subscribe()
    }

    pub async fn add_trigger(&self, trigger: UserTrigger) -> anyhow::Result<()> {
        self.service.add_trigger(trigger).await
    }

    pub async fn get_all_active_triggers(&self) -> anyhow::Result<Vec<UserTriggerExecution>> {
        self.service.get_all_active_triggers().await
    }

    pub async fn disable_triggers_before_except(
        &self,
        before: DateTime,
        excluded_ids: &[UserTriggerId],
    ) -> anyhow::Result<u64> {
        self.service.disable_triggers_before_except(before, excluded_ids).await
    }
}

impl TriggerRunner {
    pub fn new(pool: PgPool) -> Self {
        let repo = TriggerRepository::new(pool);
        let service = Arc::new(TriggerService::new(repo));
        Self { service }
    }

    pub fn client(&self) -> TriggerClient {
        TriggerClient {
            service: self.service.clone(),
        }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<TriggerEvent> {
        self.service.subscribe()
    }
}
