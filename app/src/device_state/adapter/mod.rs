pub mod db;

use crate::{
    automation::availability::ItemAvailability,
    core::timeseries::DataPoint,
    device_state::{DeviceAvailability, DeviceStateClient, DeviceStateValue},
};

#[derive(Debug, Clone, derive_more::From)]
pub enum IncomingData {
    StateValue(DataPoint<DeviceStateValue>),
    ItemAvailability(ItemAvailability),
}

pub trait IncomingDataSource<Message, Channel>
where
    Message: std::fmt::Debug,
    Channel: std::fmt::Debug,
    Self: Sized,
{
    async fn recv(&mut self) -> Option<Message>;

    fn ds_name(&self) -> &str;
    fn device_id(&self, msg: &Message) -> Option<String>;
    fn get_channels(&self, device_id: &str) -> &[Channel];

    async fn to_incoming_data(
        &self,
        device_id: &str,
        channel: &Channel,
        msg: &Message,
    ) -> anyhow::Result<Vec<IncomingData>>;
}

pub struct IncomingDataSourceRunner<M, C, S>
where
    M: std::fmt::Debug,
    C: std::fmt::Debug,
    S: IncomingDataSource<M, C>,
{
    source: S,
    device_client: DeviceStateClient,
    _marker: std::marker::PhantomData<(M, C)>,
}

impl<M, C, S> IncomingDataSourceRunner<M, C, S>
where
    M: std::fmt::Debug,
    C: std::fmt::Debug,
    S: IncomingDataSource<M, C>,
{
    pub fn new(source: S, device_client: DeviceStateClient) -> Self {
        Self {
            source,
            device_client,
            _marker: std::marker::PhantomData,
        }
    }

    pub async fn run(mut self) {
        loop {
            let msg = match self.source.recv().await {
                Some(msg) => msg,
                None => continue,
            };

            self.handle_incoming_data(&msg).await;
        }
    }

    async fn handle_incoming_data(&self, msg: &M) {
        let source = &self.source;
        let name = source.ds_name();

        let device_id = match source.device_id(msg) {
            Some(device_id) => device_id,
            None => return,
        };

        let channels = source.get_channels(&device_id);
        if channels.is_empty() {
            return;
        }

        tracing::debug!("Received {} event for devices {}: {:?}", name, device_id, channels);

        let mut incoming_data = vec![];

        for channel in channels.iter() {
            match source.to_incoming_data(&device_id, channel, msg).await {
                Ok(events) => incoming_data.extend(events),
                Err(e) => {
                    tracing::error!(
                        "Error parsing {} event for channel {:?} with payload {:?}: {:?}",
                        name,
                        channel,
                        msg,
                        e
                    );
                }
            }
        }

        for event in incoming_data.iter() {
            match event {
                IncomingData::StateValue(dp) => {
                    if let Err(e) = self.device_client.update_state(dp.clone()).await {
                        tracing::error!("Error processing state {:?}: {:?}", dp, e);
                    }
                }

                IncomingData::ItemAvailability(item) => {
                    let device_item = DeviceAvailability {
                        device_id: item.item.clone(),
                        source: item.source.clone(),
                        last_seen: item.last_seen,
                        marked_offline: item.marked_offline,
                    };

                    if let Err(e) = self.device_client.update_availability(device_item).await {
                        tracing::error!("Error processing device availability for {}: {:?}", item.item, e);
                    }
                }
            }
        }
    }
}
