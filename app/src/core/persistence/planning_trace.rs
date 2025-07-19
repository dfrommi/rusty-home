use crate::core::time::DateTime;

use sqlx::types::chrono;

use crate::core::planner::{PlanningTrace, PlanningTraceStep};

struct PlanningTraceRow {
    id: i64,
    trace_id: Option<String>,
    timestamp: chrono::DateTime<chrono::Utc>,
    steps: serde_json::Value,
}

impl TryInto<PlanningTrace> for PlanningTraceRow {
    type Error = anyhow::Error;

    fn try_into(self) -> Result<PlanningTrace, Self::Error> {
        let steps: Vec<PlanningTraceStep> = serde_json::from_value(self.steps)?;
        Ok(PlanningTrace::new(self.timestamp.into(), self.trace_id, steps))
    }
}

// Planning Trace Persistence
// Methods for storing planning execution traces
impl super::Database {
    #[tracing::instrument(skip_all)]
    pub async fn add_planning_trace(&self, result: &PlanningTrace) -> anyhow::Result<()> {
        sqlx::query!(
            r#"INSERT INTO planning_trace (trace_id, timestamp, steps) VALUES ($1, $2, $3)"#,
            result.trace_id,
            result.timestamp.into_db(),
            serde_json::to_value(result.steps.clone())?,
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}

// Planning Trace Retrieval
// Methods for querying and retrieving planning execution traces
impl super::Database {
    pub async fn get_latest_planning_trace(
        &self,
        before: crate::core::time::DateTime,
    ) -> anyhow::Result<PlanningTrace> {
        let rec = sqlx::query_as!(
            PlanningTraceRow,
            r#"SELECT * 
                FROM planning_trace
                WHERE timestamp <= $1
                ORDER BY timestamp DESC
                LIMIT 1"#,
            before.into_db()
        )
        .fetch_optional(&self.pool)
        .await?;

        match rec {
            Some(rec) => rec.try_into(),
            None => Ok(PlanningTrace::current(vec![])), //unlikely case
        }
    }

    pub async fn get_planning_traces_by_trace_id(&self, trace_id: &str) -> anyhow::Result<Option<PlanningTrace>> {
        let recs = sqlx::query_as!(
            PlanningTraceRow,
            r#"SELECT * FROM planning_trace WHERE trace_id = $1"#,
            trace_id
        )
        .fetch_optional(&self.pool)
        .await?;

        recs.map(TryInto::try_into).transpose()
    }

    pub async fn get_trace_ids(
        &self,
        range: crate::core::time::DateTimeRange,
    ) -> anyhow::Result<Vec<(String, DateTime)>> {
        let recs = sqlx::query!(
            r#"SELECT trace_id, timestamp
                FROM planning_trace
                WHERE timestamp >= $1
                AND timestamp <= $2
                ORDER BY timestamp DESC"#,
            range.start().into_db(),
            range.end().into_db(),
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(recs
            .into_iter()
            .filter_map(|rec| rec.trace_id.map(|trace_id| (trace_id, rec.timestamp.into())))
            .collect())
    }

    pub async fn get_planning_traces_in_range(
        &self,
        range: crate::core::time::DateTimeRange,
    ) -> anyhow::Result<Vec<PlanningTrace>> {
        let recs = sqlx::query_as!(
            PlanningTraceRow,
            r#"SELECT * FROM planning_trace
                WHERE timestamp >= $1
                AND timestamp <= $2
                ORDER BY timestamp DESC"#,
            range.start().into_db(),
            range.end().into_db(),
        )
        .fetch_all(&self.pool)
        .await?;

        recs.into_iter().map(TryInto::try_into).collect::<Result<Vec<_>, _>>()
    }
}
