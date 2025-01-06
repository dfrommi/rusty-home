use std::fmt::Display;

use api::command::CommandTarget;
use tokio::sync::oneshot;

use super::{resource_lock::ResourceLock, PlanningTrace};

pub struct Context<'a, A> {
    pub action: &'a A,
    pub goal_active: bool,
    pub trace: PlanningTrace,
    lock_rx: oneshot::Receiver<ResourceLock<CommandTarget>>,
    lock_tx: Option<oneshot::Sender<ResourceLock<CommandTarget>>>,
}

impl<'a, A: Display> Context<'a, A> {
    pub fn new<G: Display>(
        goal: &'a G,
        action: &'a A,
        goal_active: bool,
        lock_rx: oneshot::Receiver<ResourceLock<CommandTarget>>,
        lock_tx: oneshot::Sender<ResourceLock<CommandTarget>>,
    ) -> Self {
        let mut trace = PlanningTrace::new(action, goal);
        trace.is_goal_active = goal_active;

        Self {
            action,
            goal_active,
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
