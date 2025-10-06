use crate::core::id::ExternalId;
use crate::core::plan_for_home;
use crate::core::time::{DateTime, FIXED_NOW};
use crate::home::command::{Command, PowerToggle};
use support::TestDatabase;

use super::{infrastructure, runtime};

pub fn plan_at(iso: &str) -> Vec<(Command, ExternalId)> {
    let fake_now = DateTime::from_iso(iso).unwrap();

    let f = async {
        let command_api = TestDatabase::new();
        let api = &infrastructure().api();

        plan_for_home(api).await;

        let executed_actions = command_api.executed_actions();
        println!("TEST2: Executed actions: {executed_actions:?}");
        executed_actions
    };
    runtime().block_on(FIXED_NOW.scope(fake_now, f))
}

// #[test]
// fn test_planning() {
//     let actions = plan_at("2024-12-24T12:50:01+01:00");
//
//     for (action, source) in actions.iter() {
//         println!("{source:?} - {action:?}");
//     }
//
//     assert_eq!(actions.len(), 0);
// }
//
// #[test]
// fn test_planning_execution() {
//     let actions = plan_at("2024-12-31T19:49:07.50+01:00");
//
//     for (action, source) in actions.iter() {
//         println!("{source:?} - {action:?}");
//     }
//
//     assert_eq!(actions.len(), 1);
//     assert_eq!(
//         actions[0].0,
//         Command::SetPower {
//             device: PowerToggle::Dehumidifier,
//             power_on: true,
//         }
//     );
// }

mod support {
    use std::sync::Mutex;

    use crate::core::id::ExternalId;
    use crate::core::time::{DateTime, DateTimeRange};
    use crate::home::command::Command;

    use crate::{core::planner::PlanningTrace, home::tests::infrastructure};

    pub struct TestDatabase {
        executed_actions: Mutex<Vec<(Command, ExternalId)>>,
    }

    impl TestDatabase {
        pub fn new() -> Self {
            Self {
                executed_actions: Mutex::new(vec![]),
            }
        }
    }

    //Tracer
    impl TestDatabase {
        pub async fn add_planning_trace(&self, results: &PlanningTrace) -> anyhow::Result<()> {
            println!("{results:?}");
            Ok(())
        }

        pub async fn get_latest_planning_trace(&self, _: DateTime) -> anyhow::Result<PlanningTrace> {
            Ok(PlanningTrace::current(vec![]))
        }

        pub async fn get_planning_traces_in_range(&self, _: DateTimeRange) -> anyhow::Result<Vec<PlanningTrace>> {
            Ok(vec![])
        }

        pub async fn get_planning_traces_by_trace_id(&self, _: &str) -> anyhow::Result<Option<PlanningTrace>> {
            Ok(None)
        }

        pub async fn get_trace_ids(&self, _: DateTimeRange) -> anyhow::Result<Vec<(String, DateTime)>> {
            Ok(vec![])
        }
    }

    //Command
    impl TestDatabase {
        pub fn executed_actions(&self) -> Vec<(Command, ExternalId)> {
            self.executed_actions.lock().unwrap().clone()
        }

        pub async fn save_command(
            &self,
            command: Command,
            source: ExternalId,
            _: Option<String>,
        ) -> anyhow::Result<()> {
            println!("Pretend executing command in test: {command:?} with source: {source:?}");

            let mut executed_actions = self.executed_actions.lock().unwrap();
            executed_actions.push((command, source));

            Ok(())
        }

        pub async fn is_reflected_in_state(&self, command: &Command) -> anyhow::Result<bool> {
            command.is_reflected_in_state(&infrastructure().api()).await
        }

        pub async fn get_latest_command(
            &self,
            target: impl Into<crate::home::command::CommandTarget>,
            since: DateTime,
        ) -> anyhow::Result<Option<crate::home::command::CommandExecution>> {
            infrastructure().api().get_latest_command(target, since).await
        }

        pub async fn get_all_commands_for_target(
            &self,
            target: impl Into<crate::home::command::CommandTarget>,
            since: DateTime,
        ) -> anyhow::Result<Vec<crate::home::command::CommandExecution>> {
            infrastructure().api().get_all_commands_for_target(target, since).await
        }

        pub async fn get_all_commands(
            &self,
            from: DateTime,
            until: DateTime,
        ) -> anyhow::Result<Vec<crate::home::command::CommandExecution>> {
            infrastructure().api().get_all_commands(from, until).await
        }
    }
}
