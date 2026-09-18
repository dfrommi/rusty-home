pub mod admin;
pub mod grafana;

use std::sync::Arc;

use crate::{
    device_state::DeviceStateClient, home_state::HomeStateClient,
    observability::adapter::repository::VictoriaRepository,
};

#[derive(Clone)]
pub struct MetricsExportApi {
    repo: Arc<VictoriaRepository>,
    device_state_client: Arc<DeviceStateClient>,
    home_state_client: Arc<HomeStateClient>,
}

impl MetricsExportApi {
    pub fn new(
        repo: Arc<VictoriaRepository>,
        device_state_client: DeviceStateClient,
        home_state_client: HomeStateClient,
    ) -> Self {
        Self {
            repo,
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
            .service(grafana::routes(self.device_state_client.clone()))
    }
}
