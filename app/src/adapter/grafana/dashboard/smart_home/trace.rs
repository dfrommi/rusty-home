use std::sync::Arc;

use actix_web::web;

use crate::{
    core::HomeApi,
    adapter::grafana::{GrafanaApiError, GrafanaResponse, dashboard::TimeRangeQuery, support::csv_response},
    core::planner::PlanningTrace,
};

#[derive(Debug, Clone, serde::Deserialize)]
struct ByTraceId {
    trace_id: String,
}

pub fn routes(api: Arc<HomeApi>) -> actix_web::Scope {
    web::scope("/trace")
        .route("/", web::get().to(get_trace_ids))
        .route("/plan", web::get().to(get_trace))
        .app_data(web::Data::from(api))
}

async fn get_trace_ids(api: web::Data<HomeApi>, time_range: web::Query<TimeRangeQuery>) -> GrafanaResponse {
    #[derive(serde::Serialize)]
    struct Row {
        label: String,
        trace_id: String,
    }

    let traces = api
        .get_trace_ids(time_range.range())
        .await
        .map_err(GrafanaApiError::DataAccessError)?;

    let rows = traces.into_iter().map(|(trace_id, timestamp)| Row {
        label: format!("{timestamp} - {trace_id}"),
        trace_id,
    });

    csv_response(rows)
}

async fn get_trace(api: web::Data<HomeApi>, query: web::Query<ByTraceId>) -> GrafanaResponse {
    #[derive(serde::Serialize)]
    struct Row {
        state: String,
        action: String,
    }

    let trace: Option<PlanningTrace> = api
        .get_planning_traces_by_trace_id(&query.trace_id)
        .await
        .map_err(GrafanaApiError::DataAccessError)?;

    let views = match trace {
        Some(trace) => trace.into(),
        None => vec![],
    };

    let rows = views.into_iter().map(|trace| Row {
        state: trace.state,
        action: trace.action,
    });

    csv_response(rows)
}
