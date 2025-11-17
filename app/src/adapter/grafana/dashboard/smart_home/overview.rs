use std::sync::Arc;

use actix_web::web;
use infrastructure::TraceContext;

use crate::core::id::ExternalId;
use crate::home::command::{Command, is_user_generated};
use crate::home::state::PersistentHomeState;

use crate::{
    adapter::grafana::{GrafanaApiError, GrafanaResponse, dashboard::TimeRangeQuery, support::csv_response},
    core::HomeApi,
};

pub fn routes(api: Arc<HomeApi>) -> actix_web::Scope {
    web::scope("/overview")
        .route("/commands", web::get().to(get_commands))
        .route("/states", web::get().to(get_states))
        .route("/offline", web::get().to(get_offline_items))
        .app_data(web::Data::from(api))
}

async fn get_commands(api: web::Data<HomeApi>, time_range: web::Query<TimeRangeQuery>) -> GrafanaResponse {
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
        Command::SetPower { device, power_on } => {
            ("SetPower", device.to_string(), if *power_on { "on" } else { "off" }.to_string())
        }
        Command::SetHeating { device, target_state } => (
            "SetHeating",
            device.to_string(),
            match target_state {
                crate::home::command::HeatingTargetState::Off => "off".to_string(),
                crate::home::command::HeatingTargetState::WindowOpen => "window_open".to_string(),
                crate::home::command::HeatingTargetState::Heat {
                    temperature,
                    low_priority,
                } => {
                    format!("{} [prio {}]", temperature, if *low_priority { "low" } else { "normal" })
                }
            },
        ),
        Command::SetThermostatAmbientTemperature { device, temperature } => {
            ("SetThermostatAmbientTemperature", device.to_string(), temperature.to_string())
        }
        Command::SetThermostatLoadMean { device, value } => {
            ("SetThermostatLoadMean", device.to_string(), value.to_string())
        }
        Command::PushNotify {
            action,
            notification,
            recipient,
        } => ("PushNotify", format!("{notification} @ {recipient}"), action.to_string()),
        Command::SetEnergySaving { device, on } => (
            "SetEnergySaving",
            device.to_string(),
            if *on { "on" } else { "off" }.to_string(),
        ),
        Command::ControlFan { device, speed } => ("ControlFan", device.to_string(), speed.to_string()),
    }
}

async fn get_states(api: web::Data<HomeApi>, time_range: web::Query<TimeRangeQuery>) -> GrafanaResponse {
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
        let target = PersistentHomeState::from(&dp.value);

        let id = target.ext_id();

        Row {
            timestamp: dp.timestamp.to_human_readable(),
            type_: id.type_name().to_string(),
            item: id.variant_name().to_string(),
            value: dp.value.value().to_string(),
        }
    });

    csv_response(rows)
}

async fn get_offline_items(api: web::Data<HomeApi>) -> GrafanaResponse {
    #[derive(serde::Serialize)]
    struct Row {
        source: String,
        item: String,
        days: f64,
    }

    let offline_items = api
        .get_offline_items()
        .await
        .map_err(GrafanaApiError::DataAccessError)?;

    let rows = offline_items.into_iter().map(|item| Row {
        source: item.source,
        item: item.item,
        days: item.duration.as_days_f64(),
    });

    csv_response(rows)
}

fn source_as_string(source: &ExternalId) -> (&str, String) {
    let icon = if is_user_generated(source) { "USER" } else { "SYSTEM" };
    (icon, source.to_string())
}
