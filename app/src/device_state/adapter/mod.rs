pub mod db;
pub mod tasmota;

use crate::{
    core::timeseries::DataPoint,
    device_state::{DeviceAvailability, DeviceStateValue},
};

#[derive(Debug, Clone, derive_more::From)]
pub enum IncomingData {
    StateValue(DataPoint<DeviceStateValue>),
    ItemAvailability(DeviceAvailability),
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

    async fn recv_multi(&mut self) -> Option<Vec<IncomingData>> {
        let msg = self.recv().await?;
        let name = self.ds_name();

        let device_id = self.device_id(&msg)?;

        let channels = self.get_channels(&device_id);
        if channels.is_empty() {
            return None;
        }

        tracing::debug!("Received {} event for devices {}: {:?}", name, device_id, channels);

        let mut incoming_data = vec![];

        for channel in channels.iter() {
            match self.to_incoming_data(&device_id, channel, &msg).await {
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

        Some(incoming_data)
    }
}
