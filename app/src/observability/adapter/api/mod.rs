pub mod admin;
pub mod grafana;

use std::sync::Arc;

use crate::{
    command::CommandClient, device_state::DeviceStateClient, home_state::HomeStateClient,
    observability::adapter::repository::VictoriaRepository,
};

#[derive(Clone)]
pub struct MetricsExportApi {
    repo: Arc<VictoriaRepository>,
    command_client: Arc<CommandClient>,
    device_state_client: Arc<DeviceStateClient>,
    home_state_client: Arc<HomeStateClient>,
}

impl MetricsExportApi {
    pub fn new(
        repo: Arc<VictoriaRepository>,
        command_client: CommandClient,
        device_state_client: DeviceStateClient,
        home_state_client: HomeStateClient,
    ) -> Self {
        Self {
            repo,
            command_client: Arc::new(command_client),
            device_state_client: Arc::new(device_state_client),
            home_state_client: Arc::new(home_state_client),
        }
    }

    pub fn routes(&self) -> actix_web::Scope {
        actix_web::web::scope("/observability")
            .service(admin::routes(
                self.repo.clone(),
                self.device_state_client.clone(),
                self.home_state_client.clone(),
            ))
            .service(grafana::routes(self.command_client.clone(), self.device_state_client.clone()))
    }
}
