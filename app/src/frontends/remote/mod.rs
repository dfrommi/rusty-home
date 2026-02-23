mod adapter;
mod service;

use std::sync::Arc;

use infrastructure::Mqtt;

use crate::trigger::TriggerClient;

pub struct RemoteModule {
    service: Arc<service::RemoteService>,
    z2m_ds: adapter::z2m::Z2mRemoteIncomingDataSource,
}

impl RemoteModule {
    pub async fn new(mqtt_client: &mut Mqtt, z2m_event_topic: &str, trigger_client: TriggerClient) -> Self {
        let z2m_ds = adapter::z2m::Z2mRemoteIncomingDataSource::new(mqtt_client, z2m_event_topic).await;
        let service = Arc::new(service::RemoteService::new(trigger_client));

        Self { service, z2m_ds }
    }

    pub async fn run(mut self) {
        loop {
            let triggers = self.z2m_ds.recv_multi().await;

            match triggers {
                Some(triggers) => {
                    for trigger in triggers {
                        self.service.handle_remote_trigger(trigger).await;
                    }
                }
                None => {
                    tracing::error!("Remote z2m incoming source closed");
                    return;
                }
            }
        }
    }
}
