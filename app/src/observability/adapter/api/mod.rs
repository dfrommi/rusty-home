pub mod admin;
pub mod grafana;

use std::sync::Arc;

use crate::{
    command::CommandClient, device_state::DeviceStateClient, observability::adapter::repository::VictoriaRepository,
};

#[derive(Clone)]
pub struct MetricsExportApi {
    repo: Arc<VictoriaRepository>,
    command_client: Arc<CommandClient>,
    device_state_client: Arc<DeviceStateClient>,
}

impl MetricsExportApi {
    pub fn new(
        repo: Arc<VictoriaRepository>,
        command_client: CommandClient,
        device_state_client: DeviceStateClient,
    ) -> Self {
        Self {
            repo,
            command_client: Arc::new(command_client),
            device_state_client: Arc::new(device_state_client),
        }
    }

    pub fn routes(&self) -> actix_web::Scope {
        actix_web::web::scope("/observability")
            .service(admin::routes(self.repo.clone(), self.device_state_client.clone()))
            .service(grafana::routes(self.command_client.clone(), self.device_state_client.clone()))
    }
}
