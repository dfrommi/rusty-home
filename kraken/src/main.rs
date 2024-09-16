use settings::Settings;
use std::env;
use std::str::from_utf8;
use tracing::error;
use tracing::info;

use rumqttc::v5::{mqttbytes::v5::ConnectProperties, AsyncClient, EventLoop};
use tokio::{
    sync::mpsc::{self, Receiver, Sender},
    task::JoinSet,
};

use rumqttc::v5::mqttbytes::v5::Packet::Publish;
use rumqttc::v5::mqttbytes::QoS;
use rumqttc::v5::Event::Incoming;

use crate::adapter::{
    process_incoming_events, process_pending_commands, IncomingMessage, OutgoingMessage,
};
use api::BackendApi;
mod adapter;
mod error;
mod settings;

pub fn main() {
    let settings = Settings::new().expect("Error reading configuration");

    unsafe { env::set_var("RUST_LOG", "warn,kraken=debug") };
    tracing_subscriber::fmt::init();

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();

    rt.block_on(async {
        let db_pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(2)
            .connect(&settings.database.url)
            .await
            .unwrap();

        let mut tasks = JoinSet::new();

        let (client, eventloop) = mqtt_init(
            &settings.mqtt.host,
            settings.mqtt.port,
            &settings.mqtt.client_id,
        );

        client
            .subscribe(settings.homeassistant.topic_event, QoS::AtLeastOnce)
            .await
            .unwrap();

        let (evt_tx, evt_rx) = mpsc::channel::<IncomingMessage>(32);
        let (cmd_tx, cmd_rx) = mpsc::channel::<OutgoingMessage>(32);

        info!("Start processing messages");

        let api = BackendApi::new(db_pool);
        let evt_tx_for_init = evt_tx.clone();

        let cmd_api = api.clone();
        tasks.spawn(async move { process_pending_commands(&cmd_api, cmd_tx).await });
        tasks.spawn(async move { process_incoming_events(&api, evt_rx).await });

        tasks.spawn(async move { dispatch_mqtt_messages(eventloop, evt_tx).await });
        tasks.spawn(async move {
            send_mqtt_messages(&settings.homeassistant.topic_command, client, cmd_rx).await
        });

        adapter::init(
            &evt_tx_for_init,
            &settings.homeassistant.url,
            &settings.homeassistant.token,
        )
        .await;

        while let Some(task) = tasks.join_next().await {
            let () = task.unwrap();
        }
    });
}

async fn dispatch_mqtt_messages(mut eventloop: EventLoop, tx: Sender<IncomingMessage>) {
    loop {
        let n = eventloop.poll().await;
        match n {
            Ok(Incoming(Publish(msg))) => {
                let payload = from_utf8(&msg.payload).unwrap().to_owned();
                tx.send(IncomingMessage::HomeAssistant { payload })
                    .await
                    .unwrap();
            }
            Ok(_) => (),
            Err(err) => error!("MQTT error {:?}", err),
        }
    }
}

async fn send_mqtt_messages(topic: &str, client: AsyncClient, mut rx: Receiver<OutgoingMessage>) {
    while let Some(message) = rx.recv().await {
        match message {
            OutgoingMessage::HomeAssistant { payload } => client
                .publish(topic, QoS::ExactlyOnce, false, payload)
                .await
                .expect("Error sending command"),
        };
    }
}

fn mqtt_init(
    host: &str,
    port: u16,
    client_id: &str,
) -> (rumqttc::v5::AsyncClient, rumqttc::v5::EventLoop) {
    use rumqttc::v5::{AsyncClient, MqttOptions};

    let mut mqttoptions = MqttOptions::new(client_id, host, port);
    mqttoptions.set_keep_alive(::std::time::Duration::from_secs(5));
    mqttoptions.set_clean_start(false);

    let mut connect_props = ConnectProperties::new();
    connect_props.session_expiry_interval = 60.into();
    mqttoptions.set_connect_properties(connect_props);

    AsyncClient::new(mqttoptions, 10)
}
