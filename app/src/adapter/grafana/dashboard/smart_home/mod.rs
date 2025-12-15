use crate::{command::CommandClient, device_state::DeviceStateClient};
use actix_web::web;

mod overview;

pub fn routes(command_client: CommandClient, device_state_client: DeviceStateClient) -> actix_web::Scope {
    web::scope("/smart_home").service(overview::routes(command_client, device_state_client))
}
