use std::sync::OnceLock;

use action::HomeAction;
use goal::HomeGoal;

mod action;
pub mod command;
mod config;
mod goal;
pub mod state;
pub mod trigger;

mod common;
#[cfg(test)]
mod tests;

use crate::core::HomeApi;
pub use common::*;
pub use goal::get_active_goals;

pub struct HomePlanning;

impl HomePlanning {
    pub async fn active_goals(api: &HomeApi) -> Vec<HomeGoal> {
        get_active_goals(api).await
    }

    pub fn config() -> &'static Vec<(HomeGoal, Vec<HomeAction>)> {
        static CONFIG: OnceLock<Vec<(HomeGoal, Vec<HomeAction>)>> = OnceLock::new();
        CONFIG.get_or_init(config::default_config)
    }
}

pub mod availability {
    use crate::core::time::DateTime;

    #[derive(Debug, Clone)]
    pub struct ItemAvailability {
        pub source: String,
        pub item: String,
        pub last_seen: DateTime,
        pub marked_offline: bool,
    }
}
