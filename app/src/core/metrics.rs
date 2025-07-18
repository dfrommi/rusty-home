#![allow(dead_code)]
use crate::Infrastructure;
use crate::core::id::ExternalId;
use crate::core::{HomeApi, ValueObject};
use crate::home::state::HomeState;
use crate::port::DataPointAccess;
use infrastructure::meter::{increment, set};
use std::time::Duration;

const ITEM_TYPE: &str = "item_type";
const ITEM_NAME: &str = "item_name";
const TAG_ID: &str = "tag_id";
const OPERATION: &str = "operation";

pub fn cache_hit_data_point_access(tag_id: i64) {
    increment(
        "home_cache_hit",
        &[(OPERATION, "data_point_access"), (TAG_ID, &tag_id.to_string())],
    );
}

pub fn cache_miss_data_point_access(tag_id: i64) {
    increment(
        "home_cache_miss",
        &[(OPERATION, "data_point_access"), (TAG_ID, &tag_id.to_string())],
    );
}

pub fn start_home_state_metrics_updater(
    infrastructure: &Infrastructure,
) -> impl std::future::Future<Output = ()> + use<> {
    let api = infrastructure.api.clone();
    let mut state_changed_events = infrastructure.event_listener.new_state_changed_listener();

    async move {
        let mut interval = tokio::time::interval(Duration::from_secs(30));

        loop {
            tokio::select! {
                _ = interval.tick() => {},
                _ = state_changed_events.recv() => {},
            };

            update_home_state_metrics(&api).await;
        }
    }
}

#[tracing::instrument(skip_all)]
async fn update_home_state_metrics(api: &HomeApi) {
    for state in HomeState::variants() {
        if let Ok(data_point) = state.current_data_point(api).await {
            let value = state.to_f64(&data_point.value);
            let external_id: &ExternalId = state.as_ref();

            set(
                "home_state_value",
                value,
                &[(ITEM_TYPE, external_id.ext_type()), (ITEM_NAME, external_id.ext_name())],
            );
        }
    }
}
