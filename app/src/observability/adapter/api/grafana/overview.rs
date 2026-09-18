use std::sync::Arc;

use actix_web::web;

use crate::device_state::{DeviceStateClient, DeviceStateId};
use crate::observability::adapter::api::grafana::{GrafanaApiError, GrafanaResponse, TimeRangeQuery, csv_response};

pub fn routes(device_state_client: Arc<DeviceStateClient>) -> actix_web::Scope {
    web::scope("/overview")
        .route("/states", web::get().to(get_states))
        .app_data(web::Data::from(device_state_client))
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
        .get_all_data_points_in_range(range.clone())
        .await
        .map_err(GrafanaApiError::DataAccessError)?
        .into_iter()
        .flat_map(|(_, dps)| dps.into_iter())
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
