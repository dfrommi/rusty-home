use std::collections::HashMap;

use support::mqtt::MqttInMessage;

use super::{IncomingData, IncomingDataProcessor, IncomingMqttEventParser};

pub mod persistence;

pub struct IncomingMqttDataProcessor<C, P>
where
    P: IncomingMqttEventParser<C>,
{
    inner: P,
    rx: tokio::sync::mpsc::Receiver<MqttInMessage>,
    config: HashMap<String, Vec<C>>,
}

impl<C, P> IncomingMqttDataProcessor<C, P>
where
    P: IncomingMqttEventParser<C>,
    C: std::fmt::Debug + Clone,
{
    pub async fn new(
        parser: P,
        config: &[(&str, C)],
        mqtt_client: &mut support::mqtt::Mqtt,
    ) -> anyhow::Result<Self> {
        let mut m: HashMap<String, Vec<C>> = HashMap::new();
        for (id, channel) in config {
            let id = id.to_string();
            m.entry(id).or_default().push(channel.clone());
        }

        let rx = mqtt_client.subscribe_all(&parser.topic_patterns()).await?;

        Ok(Self {
            inner: parser,
            rx,
            config: m,
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

            let channels = match self.config.get(&device_id) {
                Some(channels) => {
                    tracing::debug!("Received event for device {}: {:?}", device_id, channels);
                    channels
                }
                None => continue,
            };

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
