use monitoring::TraceContext;
use sqlx::{types::chrono, QueryBuilder};
use support::{t, time::DateTime};

use crate::{core::planner::PlanningTrace, port::PlanningResultTracer};

#[allow(dead_code)]
struct PlanningTraceRow {
    id: i64,
    seq: i32,
    run_id: uuid::Uuid,
    action: String,
    goal: String,
    goal_active: bool,
    locked: bool,
    fulfilled: Option<bool>,
    triggered: Option<bool>,
    timestamp: chrono::DateTime<chrono::Utc>,
    correlation_id: Option<String>,
}

impl From<PlanningTraceRow> for PlanningTrace {
    fn from(val: PlanningTraceRow) -> Self {
        PlanningTrace {
            action: val.action,
            goal: val.goal,
            is_goal_active: val.goal_active,
            locked: val.locked,
            is_fulfilled: val.fulfilled,
            was_triggered: val.triggered,
            correlation_id: val.correlation_id,
        }
    }
}

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
        let recs = sqlx::query_as!(
            PlanningTraceRow,
            r#"SELECT * FROM planning_trace
                WHERE timestamp = (SELECT MAX(timestamp) FROM planning_trace WHERE timestamp <= $1)
                ORDER BY seq ASC"#,
            before.into_db()
        )
        .fetch_all(&self.pool)
        .await?;

        let result = recs.into_iter().map(Into::into).collect();

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

    async fn get_planning_traces_by_trace_id(
        &self,
        trace_id: &str,
    ) -> anyhow::Result<Vec<PlanningTrace>> {
        let recs = sqlx::query_as!(
            PlanningTraceRow,
            r#"SELECT * FROM planning_trace
                WHERE correlation_id like '%' || $1 || '%'
                ORDER BY seq ASC"#,
            trace_id
        )
        .fetch_all(&self.pool)
        .await?;

        let result = recs.into_iter().map(Into::into).collect();

        Ok(result)
    }

    async fn get_trace_ids(
        &self,
        range: support::time::DateTimeRange,
    ) -> anyhow::Result<Vec<(String, DateTime)>> {
        let recs = sqlx::query!(
            r#"SELECT DISTINCT correlation_id, timestamp
                FROM planning_trace
                WHERE timestamp >= $1
                AND timestamp <= $2
                AND timestamp <= $3
                ORDER BY correlation_id, timestamp DESC"#,
            range.start().into_db(),
            range.end().into_db(),
            t!(now).into_db(), //For timeshift in tests
        )
        .fetch_all(&self.pool)
        .await?;

        let mut result: Vec<(String, DateTime)> = recs
            .into_iter()
            .filter_map(|rec| {
                if let Some(correlation_id) = rec.correlation_id {
                    let trace_context = TraceContext::from_correlation_id(&correlation_id);
                    Some((trace_context.trace_id().to_string(), rec.timestamp.into()))
                } else {
                    None
                }
            })
            .collect();

        result.dedup_by(|a, b| a.0 == b.0);
        result.sort_by(|a, b| b.1.cmp(&a.1));

        Ok(result)
    }
}
