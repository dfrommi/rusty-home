use ::support::time::{DateTime, FIXED_NOW};
use api::command::{Command, CommandSource, PowerToggle, SetPower};
use support::TestCommandProcessor;

use crate::{
    core::planner::perform_planning,
    home::{default_config, get_active_goals},
};

use super::{infrastructure, runtime};

pub fn plan_at(iso: &str) -> Vec<(Command, CommandSource)> {
    let fake_now = DateTime::from_iso(iso).unwrap();

    let f = async {
        let tracer = support::TestPlanningResultTracer;
        let command_api = TestCommandProcessor::new(infrastructure());
        let api = &infrastructure();

        perform_planning(
            &get_active_goals(api).await,
            default_config(),
            api,
            &command_api,
            &tracer,
        )
        .await;

        command_api.executed_actions()
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
    let actions = plan_at("2024-12-25T21:45:35Z");

    for (action, source) in actions.iter() {
        println!("{:?} - {:?}", source, action);
    }

    assert_eq!(actions.len(), 1);
    assert_eq!(
        actions[0].0,
        SetPower {
            device: PowerToggle::InfraredHeater,
            power_on: false,
        }
        .into()
    );
}

mod support {
    use std::sync::Mutex;

    use api::command::{Command, CommandSource};
    use support::time::DateTime;
    use tabled::Table;

    use crate::{
        core::{planner::PlanningTrace, service::CommandState},
        home::tests::TestInfrastructure,
        port::{CommandAccess, CommandStore, PlanningResultTracer},
    };

    pub struct TestPlanningResultTracer;

    impl PlanningResultTracer for TestPlanningResultTracer {
        async fn add_planning_trace(&self, results: &[PlanningTrace]) -> anyhow::Result<()> {
            println!("{}", Table::new(results));
            //nothing to do
            Ok(())
        }
    }

    pub struct TestCommandProcessor<'a, S: CommandState<Command> + CommandAccess<Command>> {
        delegate: &'a S,
        executed_actions: Mutex<Vec<(Command, CommandSource)>>,
    }

    impl<'a, S: CommandState<Command> + CommandAccess<Command>> TestCommandProcessor<'a, S> {
        pub fn new(delegate: &'a S) -> Self {
            Self {
                delegate,
                executed_actions: Mutex::new(vec![]),
            }
        }
    }

    impl TestCommandProcessor<'_, TestInfrastructure> {
        pub fn executed_actions(&self) -> Vec<(Command, CommandSource)> {
            self.executed_actions.lock().unwrap().clone()
        }
    }

    impl CommandStore for TestCommandProcessor<'_, TestInfrastructure> {
        async fn save_command(
            &self,
            command: Command,
            source: CommandSource,
        ) -> anyhow::Result<()> {
            println!("Executing command: {:?} with source: {:?}", command, source);

            let mut executed_actions = self.executed_actions.lock().unwrap();
            executed_actions.push((command, source));

            Ok(())
        }
    }

    impl<'a> CommandState<Command> for TestCommandProcessor<'a, TestInfrastructure> {
        async fn is_reflected_in_state(&self, command: &Command) -> anyhow::Result<bool> {
            self.delegate.is_reflected_in_state(command).await
        }
    }

    impl<'a> CommandAccess<Command> for TestCommandProcessor<'a, TestInfrastructure> {
        async fn get_latest_command(
            &self,
            target: impl Into<api::command::CommandTarget>,
            since: DateTime,
        ) -> anyhow::Result<Option<api::command::CommandExecution<Command>>> {
            self.delegate.get_latest_command(target, since).await
        }

        async fn get_all_commands(
            &self,
            target: impl Into<api::command::CommandTarget>,
            since: DateTime,
        ) -> anyhow::Result<Vec<api::command::CommandExecution<Command>>> {
            self.delegate.get_all_commands(target, since).await
        }

        async fn get_latest_command_source(
            &self,
            target: impl Into<api::command::CommandTarget>,
            since: DateTime,
        ) -> anyhow::Result<Option<api::command::CommandSource>> {
            CommandAccess::<Command>::get_latest_command_source(&self.delegate, target, since).await
        }
    }
}
