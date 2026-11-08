mod adapter;
mod service;

use std::sync::Arc;

use infrastructure::Mqtt;

use crate::trigger::TriggerClient;

pub struct RemoteModule {
    service: Arc<service::RemoteService>,
    z2m_ds: adapter::z2m::Z2mRemoteIncomingDataSource,
    ha_ds: adapter::homeassistant::HomeAssistantRemoteIncomingDataSource,
}

impl RemoteModule {
    pub async fn new(
        mqtt_client: &mut Mqtt,
        z2m_event_topic: &str,
        ha_event_topic: &str,
        trigger_client: TriggerClient,
    ) -> Self {
        let z2m_ds = adapter::z2m::Z2mRemoteIncomingDataSource::new(mqtt_client, z2m_event_topic).await;
        let ha_ds =
            adapter::homeassistant::HomeAssistantRemoteIncomingDataSource::new(mqtt_client, ha_event_topic).await;
        let service = Arc::new(service::RemoteService::new(trigger_client));

        Self { service, z2m_ds, ha_ds }
    }

    pub async fn run(mut self) {
        loop {
            let (source, triggers) = tokio::select! {
                triggers = self.z2m_ds.recv_multi() => ("Z2M", triggers),
                triggers = self.ha_ds.recv_multi() => ("Home Assistant", triggers),
            };

            match triggers {
                Some(triggers) => {
                    for trigger in triggers {
                        self.service.handle_remote_trigger(trigger).await;
                    }
                }
                None => {
                    tracing::error!("Remote {} incoming source closed", source);
                    return;
                }
            }
        }
    }
}
