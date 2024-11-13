use std::{fmt::Display, sync::OnceLock};

use action::HomeAction;
use api::command::{Command, Thermostat};
use api::state::{ExternalAutoControl, RelativeHumidity, SetPoint};
use goal::{get_active_goals, HomeGoal};

mod action;
mod config;
mod goal;
mod planner;
use planner::action_ext::ExecutionAwareAction;
pub use planner::ActionResult;

#[cfg(test)]
mod tests;

use super::state::{DataPointAccess, RiskOfMould};
use super::{state::*, CommandAccess, CommandExecutor};

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
        + CommandAccess<Thermostat>
        + CommandAccess<Command>
        + CommandExecutor<Command>
        + PlanningResultTracer,
{
    let goals = get_active_goals();

    //TODO move mapping to static init
    let config = default_config()
        .iter()
        .map(|(goal, actions)| {
            (
                goal.clone(),
                actions
                    .iter()
                    .map(|a| ExecutionAwareAction::new(a.clone()))
                    .collect(),
            )
        })
        .collect::<Vec<_>>();

    planner::do_plan(&goals, &config, api).await;
}

pub trait PlanningResultTracer {
    async fn add_planning_trace<'a, A: Display>(
        &self,
        results: &[ActionResult<'a, A>],
    ) -> anyhow::Result<()>;
}
