use std::fmt::Display;

use crate::home::{command::CommandTarget, trigger::UserTriggerId};
use tokio::sync::oneshot;

use super::{resource_lock::ResourceLock, trace::PlanningTraceStep};

pub struct Context<A> {
    pub action: A,
    pub goal_active: bool,
    pub user_trigger_id: Option<UserTriggerId>,
    pub trace: PlanningTraceStep,
    lock_rx: oneshot::Receiver<ResourceLock<CommandTarget>>,
    lock_tx: Option<oneshot::Sender<ResourceLock<CommandTarget>>>,
}

impl<A: Display> Context<A> {
    pub fn new<G: Display>(
        goal: &G,
        action: A,
        goal_active: bool,
        lock_rx: oneshot::Receiver<ResourceLock<CommandTarget>>,
        lock_tx: oneshot::Sender<ResourceLock<CommandTarget>>,
    ) -> Self {
        let mut trace = PlanningTraceStep::new(&action, goal);
        trace.goal_active = goal_active;

        Self {
            action,
            goal_active,
            user_trigger_id: None,
            trace,
            lock_rx,
            lock_tx: Some(lock_tx),
        }
    }

    pub async fn get_lock(&mut self) -> anyhow::Result<ResourceLock<CommandTarget>> {
        let rx = &mut self.lock_rx;
        rx.await
            .map_err(|e| anyhow::anyhow!("Error receiving resource lock: {:?}", e))
    }

    pub async fn release_lock(&mut self, lock: ResourceLock<CommandTarget>) -> anyhow::Result<()> {
        match self.lock_tx.take() {
            Some(tx) => tx
                .send(lock)
                .map_err(|_| anyhow::anyhow!("Error sending resource lock to planner")),
            None => Ok(()),
        }
    }
}
