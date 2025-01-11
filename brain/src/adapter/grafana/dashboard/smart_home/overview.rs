use std::sync::Arc;

use actix_web::{http::header, web, HttpResponse};
use anyhow::Context;
use api::{
    command::{Command, CommandSource},
    state::Channel,
};
use monitoring::TraceContext;
use support::TypedItem;

use crate::{
    adapter::grafana::{
        dashboard::TimeRangeQuery, support::csv_response, GrafanaApiError, GrafanaResponse,
    },
    port::{CommandAccess, DataPointStore, PlanningResultTracer},
};

use super::TraceView;

pub fn routes<T>(api: Arc<T>) -> actix_web::Scope
where
    T: PlanningResultTracer + CommandAccess + DataPointStore + 'static,
{
    web::scope("/overview")
        .route("/trace", web::get().to(get_trace::<T>))
        .route("/trace/states", web::get().to(get_trace_states::<T>))
        .route("/commands", web::get().to(get_commands::<T>))
        .route("/states", web::get().to(get_states::<T>))
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
    }

    let until = *time_range.range().end();

    let traces: Vec<TraceView> = api
        .get_latest_planning_trace(until)
        .await
        .map_err(GrafanaApiError::DataAccessError)?
        .into();

    let rows = traces
        .into_iter()
        .filter_map(|trace| match trace.state.as_str() {
            "DISABLED" | "UNFULFILLED" => None,

            _ => Some(Row {
                state: trace.state,
                name: trace.name,
                target: trace.target,
            }),
        });

    csv_response(rows)
}

async fn get_trace_states<T>(
    api: web::Data<T>,
    time_range: web::Query<TimeRangeQuery>,
) -> GrafanaResponse
where
    T: PlanningResultTracer,
{
    let traces = api
        .get_planning_traces_in_range(time_range.range())
        .await
        .map_err(GrafanaApiError::DataAccessError)?;

    let header_trace = match traces.first() {
        Some(trace) => trace,
        None => return Err(GrafanaApiError::NotFound),
    };

    let mut csv = csv::Writer::from_writer(vec![]);
    let mut header: Vec<String> = vec!["timestamp".to_string()];
    for step in header_trace.steps.iter() {
        header.push(step.action.clone());
    }
    csv.serialize(&header)
        .with_context(|| "Error serializing row")
        .map_err(GrafanaApiError::InternalError)?;

    for trace in traces {
        let mut row: Vec<String> = vec![trace.timestamp.to_iso_string()];

        for step in trace.steps {
            row.push(super::trace_state(&step));
        }

        csv.serialize(&row)
            .with_context(|| "Error serializing row")
            .map_err(GrafanaApiError::InternalError)?;
    }

    Ok(HttpResponse::Ok()
        .append_header(header::ContentType(mime::TEXT_CSV))
        .body(
            csv.into_inner()
                .with_context(|| "Error creating CSV")
                .map_err(GrafanaApiError::InternalError)?,
        ))
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

async fn get_states<T>(api: web::Data<T>, time_range: web::Query<TimeRangeQuery>) -> GrafanaResponse
where
    T: DataPointStore,
{
    #[derive(serde::Serialize)]
    struct Row {
        timestamp: String,
        #[serde(rename = "type")]
        type_: String,
        item: String,
        value: String,
    }

    let range = time_range.range();
    let mut states = api
        .get_all_data_points_in_range(range.clone())
        .await
        .map_err(GrafanaApiError::DataAccessError)?;

    states.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

    let rows = states.into_iter().map(|dp| {
        let target = Channel::from(&dp.value);

        Row {
            timestamp: dp.timestamp.to_human_readable(),
            type_: target.type_name().to_string(),
            item: target.item_name().to_string(),
            value: dp.value.value_to_string(),
        }
    });

    csv_response(rows)
}

fn source_as_string(source: &CommandSource) -> (&str, String) {
    match source {
        CommandSource::System(id) => ("SYSTEM", id.to_owned()),
        CommandSource::User(id) => ("USER", id.to_owned()),
    }
}
