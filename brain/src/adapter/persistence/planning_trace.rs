use sqlx::QueryBuilder;
use support::t;

use crate::{core::planner::PlanningTrace, port::PlanningResultTracer};

impl PlanningResultTracer for super::Database {
    #[tracing::instrument(skip_all, fields(planning_traces = results.len()))]
    async fn add_planning_trace(&self, results: &[PlanningTrace]) -> anyhow::Result<()> {
        let id = sqlx::types::Uuid::from_u128(uuid::Uuid::new_v4().as_u128());
        let now = t!(now);

        let mut builder = QueryBuilder::new(
            "INSERT INTO planning_trace (run_id, seq, action, goal, goal_active, locked, fulfilled, triggered, timestamp, correlation_id) "
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
                .push_bind(now.into_db())
                .push_bind(result.correlation_id.clone());
        });

        builder.build().execute(&self.pool).await?;
        Ok(())
    }

    async fn get_latest_planning_trace(
        &self,
        before: support::time::DateTime,
    ) -> anyhow::Result<Vec<PlanningTrace>> {
        let recs = sqlx::query!(
            r#"SELECT * FROM planning_trace
                WHERE timestamp = (SELECT MAX(timestamp) FROM planning_trace WHERE timestamp <= $1)
                ORDER BY seq ASC"#,
            before.into_db()
        )
        .fetch_all(&self.pool)
        .await?;

        let result = recs
            .into_iter()
            .map(|rec| PlanningTrace {
                action: rec.action,
                goal: rec.goal,
                is_goal_active: rec.goal_active,
                locked: rec.locked,
                is_fulfilled: rec.fulfilled,
                was_triggered: rec.triggered,
                correlation_id: rec.correlation_id,
            })
            .collect();

        Ok(result)
    }

    async fn get_last_executions(
        &self,
        before: support::time::DateTime,
    ) -> anyhow::Result<Vec<(String, support::time::DateTime)>> {
        let recs = sqlx::query!(
            r#"SELECT DISTINCT ON (action) action, timestamp
                FROM planning_trace
                WHERE timestamp <= $1
                AND triggered = true
                ORDER BY action, timestamp DESC"#,
            before.into_db()
        )
        .fetch_all(&self.pool)
        .await?;

        let result = recs
            .into_iter()
            .map(|rec| (rec.action, rec.timestamp.into()))
            .collect();

        Ok(result)
    }
}
