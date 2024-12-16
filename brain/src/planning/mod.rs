use std::sync::OnceLock;

use action::HomeAction;
use api::command::{Command, EnergySavingDevice, NotificationTarget, Thermostat};
use api::state::{ExternalAutoControl, RelativeHumidity, SetPoint};
use goal::{get_active_goals, HomeGoal};

mod action;
mod config;
mod goal;
mod planner;
pub use planner::ActionResult;

#[cfg(test)]
mod tests;

use crate::port::{CommandAccess, CommandExecutor, DataPointAccess, PlanningResultTracer};

use super::state::RiskOfMould;
use super::state::*;

#[rustfmt::skip]
fn default_config() -> &'static Vec<(HomeGoal, Vec<HomeAction>)> {
    static CONFIG: OnceLock<Vec<(HomeGoal, Vec<HomeAction>)>> = OnceLock::new();
    CONFIG.get_or_init(|| { config::default_config() })
}

pub async fn plan_for_home<T>(api: &T)
where
    T: DataPointAccess<Powered>
        + DataPointAccess<ExternalAutoControl>
        + DataPointAccess<SetPoint>
        + DataPointAccess<RiskOfMould>
        + DataPointAccess<ColdAirComingIn>
        + DataPointAccess<Opened>
        + DataPointAccess<AutomaticTemperatureIncrease>
        + DataPointAccess<UserControlled>
        + DataPointAccess<RelativeHumidity>
        + DataPointAccess<Resident>
        + CommandAccess<Command>
        + CommandAccess<Thermostat>
        + CommandAccess<NotificationTarget>
        + CommandAccess<EnergySavingDevice>
        + CommandExecutor<Command>
        + PlanningResultTracer,
{
    let goals = get_active_goals();

    planner::do_plan(&goals, default_config(), api).await;
}
