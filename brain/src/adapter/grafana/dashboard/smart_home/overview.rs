use std::sync::Arc;

use actix_web::web;
use api::command::{Command, CommandSource};
use monitoring::TraceContext;

use crate::{
    adapter::grafana::{
        dashboard::TimeRangeQuery, support::csv_response, GrafanaApiError, GrafanaResponse,
    },
    port::{CommandAccess, PlanningResultTracer},
};

use super::TraceView;

pub fn routes<T>(api: Arc<T>) -> actix_web::Scope
where
    T: PlanningResultTracer + CommandAccess + 'static,
{
    web::scope("/overview")
        .route("/trace", web::get().to(get_trace::<T>))
        .route("/commands", web::get().to(get_commands::<T>))
        .app_data(web::Data::from(api))
}

async fn get_trace<T>(api: web::Data<T>, time_range: web::Query<TimeRangeQuery>) -> GrafanaResponse
where
    T: PlanningResultTracer,
{
    #[derive(serde::Serialize)]
    struct Row {
        state: String,
        name: String,
        target: Option<String>,
        last_triggered: Option<String>,
    }

    let until = *time_range.range().end();

    let traces = api
        .get_latest_planning_trace(until)
        .await
        .map_err(GrafanaApiError::DataAccessError)?
        .into_iter()
        .map(|t| t.into())
        .collect::<Vec<TraceView>>();

    let last_executions = api
        .get_last_executions(until)
        .await
        .map_err(GrafanaApiError::DataAccessError)?;

    let rows = traces.into_iter().filter_map(|trace| {
        let last_execution = last_executions
            .iter()
            .find(|(a, _)| a == &trace.action)
            .map(|(_, timestamp)| *timestamp);
        let last_execution = last_execution.map(|dt| dt.to_human_readable());

        match trace.state.as_str() {
            "DISABLED" | "UNFULFILLED" => None,

            _ => Some(Row {
                state: trace.state,
                name: trace.name,
                target: trace.target,
                last_triggered: last_execution,
            }),
        }
    });

    csv_response(rows)
}

async fn get_commands<T>(
    api: web::Data<T>,
    time_range: web::Query<TimeRangeQuery>,
) -> GrafanaResponse
where
    T: CommandAccess,
{
    #[derive(serde::Serialize)]
    struct Row {
        icon: String,
        timestamp: String,
        r#type: String,
        target: String,
        state: String,
        source: String,
        trace_id: Option<String>,
    }

    let range = time_range.range();
    let mut commands = api
        .get_all_commands(*range.start(), *range.end())
        .await
        .map_err(GrafanaApiError::DataAccessError)?;

    commands.sort_by(|a, b| b.created.cmp(&a.created));

    let rows = commands.into_iter().map(|cmd| {
        let (command_type, target, state) = command_as_string(&cmd.command);
        let (icon, source) = source_as_string(&cmd.source);

        let trace_id = cmd
            .correlation_id
            .map(|id| TraceContext::from_correlation_id(&id).trace_id());

        Row {
            icon: icon.to_string(),
            timestamp: cmd.created.to_human_readable(),
            r#type: command_type.to_string(),
            target,
            state,
            source,
            trace_id,
        }
    });

    csv_response(rows)
}

fn command_as_string(command: &Command) -> (&str, String, String) {
    match command {
        Command::SetPower { device, power_on } => (
            "SetPower",
            device.to_string(),
            if *power_on { "on" } else { "off" }.to_string(),
        ),
        Command::SetHeating {
            device,
            target_state,
        } => (
            "SetHeating",
            device.to_string(),
            match target_state {
                api::command::HeatingTargetState::Auto => "auto".to_string(),
                api::command::HeatingTargetState::Off => "off".to_string(),
                api::command::HeatingTargetState::Heat { temperature, .. } => {
                    temperature.to_string()
                }
            },
        ),
        Command::PushNotify {
            action,
            notification,
            recipient,
        } => (
            "PushNotify",
            format!("{} @ {}", notification, recipient),
            action.to_string(),
        ),
        Command::SetEnergySaving { device, on } => (
            "SetEnergySaving",
            device.to_string(),
            if *on { "on" } else { "off" }.to_string(),
        ),
    }
}

fn source_as_string(source: &CommandSource) -> (&str, String) {
    match source {
        CommandSource::System(id) => ("SYSTEM", id.to_owned()),
        CommandSource::User(id) => ("USER", id.to_owned()),
    }
}
