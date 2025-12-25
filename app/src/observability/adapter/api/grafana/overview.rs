use actix_web::web;
use infrastructure::TraceContext;

use crate::command::{Command, CommandClient, HeatingTargetState};
use crate::device_state::{DeviceStateClient, DeviceStateId};
use crate::observability::adapter::api::grafana::{GrafanaApiError, GrafanaResponse, TimeRangeQuery, csv_response};

pub fn routes(command_client: CommandClient, device_state_client: DeviceStateClient) -> actix_web::Scope {
    web::scope("/overview")
        .route("/commands", web::get().to(get_commands))
        .route("/states", web::get().to(get_states))
        .route("/offline", web::get().to(get_offline_items))
        .app_data(web::Data::new(command_client))
        .app_data(web::Data::new(device_state_client))
}

async fn get_commands(
    command_client: web::Data<CommandClient>,
    time_range: web::Query<TimeRangeQuery>,
) -> GrafanaResponse {
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
    let mut commands = command_client
        .get_all_commands(*range.start(), *range.end())
        .await
        .map_err(GrafanaApiError::DataAccessError)?;

    commands.sort_by(|a, b| b.created.cmp(&a.created));

    let rows = commands.into_iter().map(|cmd| {
        let (command_type, target, state) = command_as_string(&cmd.command);
        let source = cmd.source.to_string();
        let icon = if cmd.is_user_generated() { "USER" } else { "SYSTEM" };

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
        Command::SetThermostatValveOpeningPosition { device, value } => {
            ("SetThermostatValveOpeningPosition", device.to_string(), value.to_string())
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

async fn get_states(
    device_client: web::Data<DeviceStateClient>,
    time_range: web::Query<TimeRangeQuery>,
) -> GrafanaResponse {
    #[derive(serde::Serialize)]
    struct Row {
        timestamp: String,
        #[serde(rename = "type")]
        type_: String,
        item: String,
        value: String,
    }

    let range = time_range.range();
    let mut states = device_client
        .get_all_data_points_in_range_strictly(range.clone())
        .await
        .map_err(GrafanaApiError::DataAccessError)?
        .into_iter()
        .filter(|dp| dp.timestamp >= *range.start())
        .collect::<Vec<_>>();

    states.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

    let rows = states.into_iter().map(|dp| {
        let target = DeviceStateId::from(&dp.value);
        let id = target.ext_id();
        let fvalue = f64::from(&dp.value);

        Row {
            timestamp: dp.timestamp.to_human_readable(),
            type_: id.type_name().to_string(),
            item: id.variant_name().to_string(),
            //TODO implement proper formatting again
            value: format!("{fvalue}"),
        }
    });

    csv_response(rows)
}

async fn get_offline_items(client: web::Data<DeviceStateClient>) -> GrafanaResponse {
    #[derive(serde::Serialize)]
    struct Row {
        source: String,
        item: String,
        days: f64,
    }

    let offline_items = client
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
