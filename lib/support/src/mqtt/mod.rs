use std::str::{from_utf8, Utf8Error};

use rumqttc::v5::{
    mqttbytes::{
        v5::{ConnectProperties, Publish, SubscribeProperties},
        QoS,
    },
    AsyncClient, EventLoop, MqttOptions,
};

use rumqttc::v5::Event::Incoming;

use tokio::{
    sync::mpsc::{self, Receiver, Sender},
    task::JoinSet,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MqttOutMessage {
    pub topic: String,
    pub payload: String,
    pub retain: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MqttInMessage {
    pub topic: String,
    pub payload: String,
}

pub struct Mqtt {
    client: AsyncClient,
    event_loop: EventLoop,
    subsciptions: Vec<Sender<MqttInMessage>>,
    publisher_tx: Sender<MqttOutMessage>,
    publisher_rx: Receiver<MqttOutMessage>,
}

impl Mqtt {
    pub fn connect(host: &str, port: u16, client_id: &str) -> Self {
        let mut mqttoptions = MqttOptions::new(client_id, host, port);
        mqttoptions.set_keep_alive(::std::time::Duration::from_secs(5));
        mqttoptions.set_clean_start(false);

        let mut connect_props = ConnectProperties::new();
        connect_props.session_expiry_interval = 60.into();
        mqttoptions.set_connect_properties(connect_props);

        let (client, event_loop) = AsyncClient::new(mqttoptions, 10);
        let (pub_tx, pub_rx) = mpsc::channel::<MqttOutMessage>(32);

        Mqtt {
            client,
            event_loop,
            subsciptions: vec![],
            publisher_rx: pub_rx,
            publisher_tx: pub_tx,
        }
    }

    pub async fn subscribe(
        &mut self,
        topic: impl Into<String>,
    ) -> Result<Receiver<MqttInMessage>, rumqttc::v5::ClientError> {
        self.client
            .subscribe_with_properties(
                topic,
                QoS::AtLeastOnce,
                SubscribeProperties {
                    id: Some(self.subsciptions.len()),
                    user_properties: vec![],
                },
            )
            .await?;

        let (tx, rx) = mpsc::channel::<MqttInMessage>(32);
        self.subsciptions.push(tx);

        Ok(rx)
    }

    pub fn new_publisher(&self) -> Sender<MqttOutMessage> {
        self.publisher_tx.clone()
    }

    pub async fn process(mut self) {
        let mut tasks = JoinSet::new();

        let client = self.client;
        let mut event_loop = self.event_loop;

        tasks.spawn(async move {
            loop {
                match event_loop.poll().await {
                    Ok(Incoming(rumqttc::v5::mqttbytes::v5::Packet::Publish(msg))) => {
                        let mqtt_in_message: MqttInMessage = match (&msg).try_into() {
                            Ok(m) => m,
                            Err(e) => {
                                tracing::error!("Error parsing MQTT message: {}", e);
                                continue;
                            }
                        };

                        let subscription_ids = match msg.properties {
                            Some(p) => p.subscription_identifiers,
                            None => {
                                tracing::error!("No subscription identifiers in MQTT message");
                                continue;
                            }
                        };

                        for id in subscription_ids {
                            match self.subsciptions.get(id) {
                                Some(tx) => {
                                    if let Err(e) = tx.send(mqtt_in_message.clone()).await {
                                        tracing::error!(
                                            "Failed to forward MQTT message to subscriber: {}",
                                            e
                                        );
                                    }
                                }
                                None => {
                                    tracing::error!("No subscription for id: {}", id);
                                }
                            }
                        }
                    }
                    Ok(_) => {}
                    Err(e) => {
                        tracing::error!("MQTT error: {}", e);
                    }
                }
            }
        });

        tasks.spawn(async move {
            while let Some(cmd) = self.publisher_rx.recv().await {
                if let Err(e) = client
                    .publish(cmd.topic.clone(), QoS::ExactlyOnce, cmd.retain, cmd.payload)
                    .await
                {
                    tracing::error!("Error publishing MQTT message to {}: {}", cmd.topic, e);
                }
            }
        });

        while let Some(task) = tasks.join_next().await {
            let () = task.unwrap();
        }
    }
}

impl TryInto<MqttInMessage> for &Publish {
    type Error = Utf8Error;

    fn try_into(self) -> Result<MqttInMessage, Self::Error> {
        Ok(MqttInMessage {
            topic: from_utf8(&self.topic)?.to_string(),
            payload: from_utf8(&self.payload)?.to_string(),
        })
    }
}
