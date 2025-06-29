use std::collections::HashMap;

use infrastructure::MqttInMessage;

use crate::Database;

use super::{IncomingData, IncomingDataProcessor, IncomingMqttEventParser};

pub mod persistence;

pub struct DeviceConfig<V> {
    config: HashMap<String, Vec<V>>,
}

impl<V> DeviceConfig<V>
where
    V: Clone,
{
    pub fn new(config: &[(&str, V)]) -> Self {
        let mut m: HashMap<String, Vec<V>> = HashMap::new();
        for (key, value) in config {
            let key = key.to_string();
            m.entry(key).or_default().push(value.clone());
        }

        Self { config: m }
    }

    pub fn get(&self, key: &str) -> &[V] {
        match self.config.get(key) {
            Some(v) => v,
            None => &[],
        }
    }
}

//TODO OutgoingDataSender for commands and Homekit state

pub trait IncomingDataSource<Message, Channel> {
    async fn recv_missed_on_startup(&mut self) -> Vec<Message> {
        vec![]
    }

    async fn recv(&mut self) -> Option<Message>;

    fn device_id(&self, msg: &Message) -> Option<String>;
    fn get_channels(&self, device_id: &str) -> &[Channel];

    fn to_incoming_data(
        &self,
        device_id: &str,
        channel: &Channel,
        msg: &Message,
    ) -> anyhow::Result<Vec<IncomingData>>;
}

pub async fn process_incoming_data_source<M, C>(
    name: &str,
    mut source: impl IncomingDataSource<M, C>,
    db: &Database,
) -> anyhow::Result<()>
where
    M: std::fmt::Debug,
    C: std::fmt::Debug,
{
    for msg in source.recv_missed_on_startup().await {
        handle_incoming_data(name, &msg, &source, db).await;
    }

    loop {
        let msg = match source.recv().await {
            Some(msg) => msg,
            None => continue,
        };

        handle_incoming_data(name, &msg, &source, db).await;
    }
}

async fn handle_incoming_data<M, C>(
    name: &str,
    msg: &M,
    source: &impl IncomingDataSource<M, C>,
    db: &Database,
) where
    M: std::fmt::Debug,
    C: std::fmt::Debug,
{
    let device_id = match source.device_id(msg) {
        Some(device_id) => device_id,
        None => return,
    };

    let channels = source.get_channels(&device_id);
    if channels.is_empty() {
        return;
    }

    tracing::debug!(
        "Received {} event for devices {}: {:?}",
        name,
        device_id,
        channels
    );

    let mut incoming_data = vec![];

    for channel in channels.iter() {
        match source.to_incoming_data(&device_id, channel, msg) {
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
                if let Err(e) = db.add_state(&dp.value, &dp.timestamp).await {
                    tracing::error!("Error processing state {:?}: {:?}", dp, e);
                }
            }

            IncomingData::UserTrigger(trigger) => {
                if let Err(e) = db.add_user_trigger(trigger.clone()).await {
                    tracing::error!("Error processing user trigger {:?}: {:?}", trigger, e);
                }
            }

            IncomingData::ItemAvailability(item) => {
                if let Err(e) = db.add_item_availability(item.clone()).await {
                    tracing::error!("Error processing item availability {:?}: {:?}", item, e);
                }
            }
        }
    }
}

pub struct IncomingMqttDataProcessor<C, P>
where
    P: IncomingMqttEventParser<C>,
{
    inner: P,
    rx: tokio::sync::mpsc::Receiver<MqttInMessage>,
    config: DeviceConfig<C>,
}

impl<C, P> IncomingMqttDataProcessor<C, P>
where
    P: IncomingMqttEventParser<C>,
    C: std::fmt::Debug + Clone,
{
    pub async fn new(
        parser: P,
        config: &[(&str, C)],
        mqtt_client: &mut infrastructure::Mqtt,
    ) -> anyhow::Result<Self> {
        let rx = mqtt_client.subscribe_all(&parser.topic_patterns()).await?;

        Ok(Self {
            inner: parser,
            rx,
            config: DeviceConfig::new(config),
        })
    }
}

impl<C, P> IncomingDataProcessor for IncomingMqttDataProcessor<C, P>
where
    P: IncomingMqttEventParser<C>,
    C: std::fmt::Debug + Clone,
{
    async fn process(
        &mut self,
        sender: tokio::sync::mpsc::Sender<IncomingData>,
    ) -> anyhow::Result<()> {
        loop {
            let msg = match self.rx.recv().await {
                Some(msg) => msg,
                None => {
                    anyhow::bail!("Event receiver closed");
                }
            };

            let device_id = match self.inner.device_id(&msg) {
                Some(device_id) => device_id,
                None => continue,
            };

            let channels = self.config.get(&device_id);
            if channels.is_empty() {
                continue;
            }

            tracing::debug!("Received event for devices {}: {:?}", device_id, channels);

            let mut incoming_data = vec![];

            for channel in channels {
                match self.inner.get_events(&device_id, channel, &msg) {
                    Ok(events) => incoming_data.extend(events),
                    Err(e) => {
                        tracing::error!(
                            "Error parsing event for channel {:?} with payload {}: {:?}",
                            channel,
                            msg.payload,
                            e
                        );
                    }
                }
            }

            for event in incoming_data {
                if let Err(e) = sender.send(event.clone()).await {
                    tracing::error!("Error sending event {:?}: {:?}", event, e);
                }
            }
        }
    }
}
