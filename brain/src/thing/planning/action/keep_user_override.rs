use std::fmt::Display;

use super::{Action, Resource};
use crate::thing::{DataPointAccess, UserControlled};
use anyhow::Result;

#[derive(Debug, Clone)]
pub struct KeepUserOverride(UserControlled, Resource);

impl KeepUserOverride {
    pub fn new(user_controlled: UserControlled, resource: Resource) -> Self {
        Self(user_controlled, resource)
    }
}

impl Action for KeepUserOverride {
    async fn preconditions_fulfilled(&self) -> Result<bool> {
        self.0.current().await
    }

    //TODO avoid duplicate call and potential issue around time gap
    async fn is_running(&self) -> Result<bool> {
        self.preconditions_fulfilled().await
    }

    async fn start(&self) -> Result<()> {
        anyhow::bail!("User controlled action {} should never be started", self);
    }

    async fn stop(&self) -> Result<()> {
        anyhow::bail!("User controlled action {} should never be stopped", self);
    }

    fn controls_resource(&self) -> Option<Resource> {
        Some(self.1.clone())
    }
}

impl Display for KeepUserOverride {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "KeepUserOverride[{}]", self.0)
    }
}
