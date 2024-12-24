use sqlx::{PgPool, QueryBuilder};
use support::t;

use crate::{core::planner::PlanningTrace, port::PlanningResultTracer};

impl<DB: AsRef<PgPool>> PlanningResultTracer for DB {
    async fn add_planning_trace(&self, results: &[PlanningTrace]) -> anyhow::Result<()> {
        let id = sqlx::types::Uuid::from_u128(uuid::Uuid::new_v4().as_u128());

        let mut builder = QueryBuilder::new(
            "INSERT INTO planning_trace (run_id, seq, action, goal, goal_active, locked, fulfilled, triggered, timestamp) "
        );

        builder.push_values(results.iter().enumerate(), |mut b, (i, result)| {
            b.push_bind(id.to_owned())
                .push_bind((i + 1) as i32)
                .push_bind(result.action.as_str())
                .push_bind(result.goal.as_str())
                .push_bind(result.is_goal_active)
                .push_bind(result.locked)
                .push_bind(result.is_fulfilled)
                .push_bind(result.was_triggered)
                .push_bind(t!(now).into_db());
        });

        builder.build().execute(self.as_ref()).await?;
        Ok(())
    }
}
