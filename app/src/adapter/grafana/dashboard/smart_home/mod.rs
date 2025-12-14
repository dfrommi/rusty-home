use std::sync::Arc;

use crate::{core::HomeApi, device_state::DeviceStateClient};
use actix_web::web;

mod overview;

pub fn routes(api: Arc<HomeApi>, device_state_client: Arc<DeviceStateClient>) -> actix_web::Scope {
    web::scope("/smart_home").service(overview::routes(api.clone(), device_state_client.clone()))
}
