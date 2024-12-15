use std::fmt::Display;

use sqlx::{PgPool, QueryBuilder};
use support::t;

use crate::{planning::ActionResult, port::PlanningResultTracer};

impl<DB: AsRef<PgPool>> PlanningResultTracer for DB {
    async fn add_planning_trace<'a, A: Display>(
        &self,
        results: &[ActionResult<'a, A>],
    ) -> anyhow::Result<()> {
        let id = sqlx::types::Uuid::from_u128(uuid::Uuid::new_v4().as_u128());

        let mut builder = QueryBuilder::new(
            "INSERT INTO action_plan_log (run_id, seq, action, should_be_started, should_be_stopped, goal_active, locked, fulfilled, running, timestamp) "
        );

        builder.push_values(results.iter().enumerate(), |mut b, (i, result)| {
            b.push_bind(id.to_owned())
                .push_bind((i + 1) as i32)
                .push_bind(format!("{}", result.action))
                .push_bind(result.should_be_started)
                .push_bind(result.should_be_stopped)
                .push_bind(result.is_goal_active)
                .push_bind(result.locked)
                .push_bind(result.is_fulfilled)
                .push_bind(result.is_running)
                .push_bind(t!(now).into_db());
        });

        builder.build().execute(self.as_ref()).await?;

        Ok(())
    }
}
