use ::support::time::{DateTime, FIXED_NOW};
use api::command::{Command, CommandSource, PowerToggle};
use support::TestCommandProcessor;

use crate::home::plan_for_home;

use super::{infrastructure, runtime};

pub fn plan_at(iso: &str) -> Vec<(Command, CommandSource)> {
    let fake_now = DateTime::from_iso(iso).unwrap();

    let f = async {
        let tracer = support::TestPlanningResultTracer;
        let command_api = TestCommandProcessor::new();
        let api = &infrastructure().api();

        plan_for_home(api, &command_api, &tracer).await;

        let executed_actions = command_api.executed_actions();
        println!("TEST2: Executed actions: {:?}", executed_actions);
        executed_actions
    };
    runtime().block_on(FIXED_NOW.scope(fake_now, f))
}

#[test]
fn test_planning() {
    let actions = plan_at("2024-12-24T12:50:01+01:00");

    for (action, source) in actions.iter() {
        println!("{:?} - {:?}", source, action);
    }

    assert_eq!(actions.len(), 0);
}

#[test]
fn test_planning_execution() {
    let actions = plan_at("2024-12-31T19:49:07.50+01:00");

    for (action, source) in actions.iter() {
        println!("{:?} - {:?}", source, action);
    }

    assert_eq!(actions.len(), 1);
    assert_eq!(
        actions[0].0,
        Command::SetPower {
            device: PowerToggle::Dehumidifier,
            power_on: true,
        }
    );
}

mod support {
    use std::sync::Mutex;

    use api::command::{Command, CommandSource};
    use support::time::DateTime;
    use tabled::Table;

    use crate::{
        core::{planner::PlanningTrace, service::CommandState},
        home::tests::infrastructure,
        port::{CommandAccess, CommandStore, PlanningResultTracer},
    };

    pub struct TestPlanningResultTracer;

    impl PlanningResultTracer for TestPlanningResultTracer {
        async fn add_planning_trace(&self, results: &[PlanningTrace]) -> anyhow::Result<()> {
            println!("{}", Table::new(results));
            //nothing to do
            Ok(())
        }

        async fn get_latest_planning_trace(
            &self,
            _: DateTime,
        ) -> anyhow::Result<Vec<PlanningTrace>> {
            Ok(vec![])
        }

        async fn get_last_executions(
            &self,
            _: DateTime,
        ) -> anyhow::Result<Vec<(String, DateTime)>> {
            Ok(vec![])
        }
    }

    pub struct TestCommandProcessor {
        executed_actions: Mutex<Vec<(Command, CommandSource)>>,
    }

    impl TestCommandProcessor {
        pub fn new() -> Self {
            Self {
                executed_actions: Mutex::new(vec![]),
            }
        }
    }

    impl TestCommandProcessor {
        pub fn executed_actions(&self) -> Vec<(Command, CommandSource)> {
            self.executed_actions.lock().unwrap().clone()
        }
    }

    impl CommandStore for TestCommandProcessor {
        async fn save_command(
            &self,
            command: Command,
            source: CommandSource,
        ) -> anyhow::Result<()> {
            println!(
                "Pretend executing command in test: {:?} with source: {:?}",
                command, source
            );

            let mut executed_actions = self.executed_actions.lock().unwrap();
            executed_actions.push((command, source));

            Ok(())
        }
    }

    impl CommandState for TestCommandProcessor {
        async fn is_reflected_in_state(&self, command: &Command) -> anyhow::Result<bool> {
            infrastructure().api().is_reflected_in_state(command).await
        }
    }

    impl CommandAccess for TestCommandProcessor {
        async fn get_latest_command(
            &self,
            target: impl Into<api::command::CommandTarget>,
            since: DateTime,
        ) -> anyhow::Result<Option<api::command::CommandExecution>> {
            infrastructure()
                .api()
                .get_latest_command(target, since)
                .await
        }

        async fn get_all_commands_for_target(
            &self,
            target: impl Into<api::command::CommandTarget>,
            since: DateTime,
        ) -> anyhow::Result<Vec<api::command::CommandExecution>> {
            infrastructure()
                .api()
                .get_all_commands_for_target(target, since)
                .await
        }

        async fn get_all_commands(
            &self,
            from: DateTime,
            until: DateTime,
        ) -> anyhow::Result<Vec<api::command::CommandExecution>> {
            infrastructure().api().get_all_commands(from, until).await
        }
    }
}
