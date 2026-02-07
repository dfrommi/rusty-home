use std::sync::Arc;

use rumqttc::v5::{
    AsyncClient, EventLoop, MqttOptions,
    mqttbytes::{
        QoS,
        v5::{ConnectProperties, SubscribeProperties},
    },
};

use rumqttc::v5::Event::Incoming;
use tokio::sync::mpsc;

use super::*;

pub struct Mqtt {
    client: Arc<AsyncClient>,
    event_loop: EventLoop,
    subsciptions: Vec<MqttSubscriptionHandle>,
}

struct MqttSubscriptionHandle {
    topic: String,
    txs: Vec<mpsc::Sender<MqttInMessage>>,
}

impl Mqtt {
    pub fn connect(host: &str, port: u16, client_id: &str) -> Self {
        let mut mqttoptions = MqttOptions::new(client_id, host, port);
        mqttoptions.set_keep_alive(::std::time::Duration::from_secs(5));
        mqttoptions.set_clean_start(false);

        let mut connect_props = ConnectProperties::new();
        connect_props.session_expiry_interval = 60.into();
        connect_props.max_packet_size = Some(1024 * 1024);
        mqttoptions.set_connect_properties(connect_props);

        let (client, event_loop) = AsyncClient::new(mqttoptions, 10);

        Mqtt {
            client: Arc::new(client),
            event_loop,
            subsciptions: vec![],
        }
    }

    pub async fn subscribe(&mut self, topic: impl Into<String>) -> anyhow::Result<MqttSubscription> {
        self.subscribe_all(&[topic.into()]).await
    }

    pub async fn subscribe_all(&mut self, topic: &[String]) -> anyhow::Result<MqttSubscription> {
        let (tx, rx) = mpsc::channel::<MqttInMessage>(32);

        for topic in topic {
            if let Some(subscription) = self.subsciptions.iter_mut().find(|s| s.topic == *topic) {
                tracing::info!("Adding subscription to already exsinging subscription: {:?}", &topic);

                subscription.txs.push(tx.clone());
                continue;
            };

            tracing::info!("Creating new subscription for topic: {:?}", &topic);

            let subscription = MqttSubscriptionHandle {
                topic: topic.clone(),
                txs: vec![tx.clone()],
            };

            self.subsciptions.push(subscription);

            self.client
                .subscribe_with_properties(
                    topic,
                    QoS::AtLeastOnce,
                    SubscribeProperties {
                        id: Some(self.subsciptions.len()), //must be > 0
                        user_properties: vec![],
                    },
                )
                .await?;
        }

        Ok(MqttSubscription::new(rx))
    }

    pub fn sender(&self, base_topic: impl Into<String>) -> MqttSender {
        MqttSender::new(self.client.clone(), base_topic)
    }

    pub async fn run(mut self) {
        //Receive and forward MQTT messages
        loop {
            match self.event_loop.poll().await {
                Ok(Incoming(rumqttc::v5::mqttbytes::v5::Packet::Publish(publish))) => {
                    self.handle_publish(publish).await;
                }
                Ok(_) => {}
                Err(e) => {
                    tracing::error!("MQTT error: {}", e);
                }
            }
        }
    }

    async fn handle_publish(&self, msg: rumqttc::v5::mqttbytes::v5::Publish) {
        let mqtt_in_message: MqttInMessage = match (&msg).try_into() {
            Ok(m) => m,
            Err(e) => {
                tracing::error!("Error parsing MQTT message: {}", e);
                return;
            }
        };

        tracing::trace!("Received MQTT message on topic {}", mqtt_in_message.topic,);

        let subscription_ids = match msg.properties {
            Some(p) => p.subscription_identifiers,
            None => {
                tracing::error!("No subscription identifiers in MQTT message");
                return;
            }
        };

        for id in subscription_ids {
            match self.subsciptions.get(id - 1) {
                Some(sub) => {
                    for tx in sub.txs.iter() {
                        tracing::trace!(
                            "Forwarding MQTT message to subscriber {} (closed={}): {:?}",
                            sub.topic,
                            tx.is_closed(),
                            mqtt_in_message
                        );
                        if let Err(e) = tx
                            .send_timeout(mqtt_in_message.clone(), tokio::time::Duration::from_secs(5))
                            .await
                        {
                            tracing::error!("Failed to forward MQTT message to subscriber {}: {}", sub.topic, e);
                        }
                    }
                }
                None => {
                    tracing::error!("No subscription for id: {}", id);
                }
            }
        }
    }
}
