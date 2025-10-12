use crate::core::HomeApi;

use super::IncomingData;

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

    async fn run(mut self, api: &HomeApi) {
        loop {
            let msg = match self.recv().await {
                Some(msg) => msg,
                None => continue,
            };

            handle_incoming_data(&msg, &self, api).await;
        }
    }
}

async fn handle_incoming_data<M, C>(msg: &M, source: &impl IncomingDataSource<M, C>, api: &HomeApi)
where
    M: std::fmt::Debug,
    C: std::fmt::Debug,
{
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
                if let Err(e) = api.add_state(&dp.value, &dp.timestamp).await {
                    tracing::error!("Error processing state {:?}: {:?}", dp, e);
                }
            }

            IncomingData::UserTrigger(trigger) => {
                if let Err(e) = api.add_user_trigger(trigger.clone()).await {
                    tracing::error!("Error processing user trigger {:?}: {:?}", trigger, e);
                }
            }

            IncomingData::ItemAvailability(item) => {
                if let Err(e) = api.add_item_availability(item.clone()).await {
                    tracing::error!("Error processing item availability {:?}: {:?}", item, e);
                }
            }
        }
    }
}
