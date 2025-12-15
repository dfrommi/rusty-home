pub mod energy_meter;
pub mod grafana;
pub mod homeassistant;
pub mod homekit;
pub mod mcp;
pub mod metrics_export;

pub use command::CommandExecutorRunner;
pub use incoming::IncomingDataSourceRunner;

mod incoming {
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
}

mod command {
    use infrastructure::TraceContext;

    use crate::command::{Command, CommandClient, CommandExecution};

    pub trait CommandExecutor {
        //Returns true if command was executed
        async fn execute_command(&self, command: &Command) -> anyhow::Result<bool>;
    }

    pub struct CommandExecutorRunner<E: CommandExecutor> {
        executor: E,
        pending_command_rx: tokio::sync::broadcast::Receiver<CommandExecution>,
        command_client: CommandClient,
    }

    impl<E: CommandExecutor> CommandExecutorRunner<E> {
        pub fn new(
            executor: E,
            pending_command_rx: tokio::sync::broadcast::Receiver<CommandExecution>,
            command_client: CommandClient,
        ) -> Self {
            Self {
                executor,
                pending_command_rx,
                command_client,
            }
        }

        pub async fn run(mut self) {
            loop {
                let msg = match self.pending_command_rx.recv().await {
                    Ok(msg) => msg,
                    Err(e) => {
                        tracing::warn!("Error consuming command-added event: {}", e);
                        continue;
                    }
                };

                process_command(msg, &self.command_client, &self.executor).await;
            }
        }
    }

    #[tracing::instrument(skip_all, fields(command = ?cmd.command))]
    async fn process_command(cmd: CommandExecution, command_client: &CommandClient, executor: &impl CommandExecutor) {
        TraceContext::continue_from(&cmd.correlation_id);

        let res = executor.execute_command(&cmd.command).await;

        handle_execution_result(cmd.id, res, command_client).await;
    }

    async fn handle_execution_result(command_id: i64, res: anyhow::Result<bool>, command_client: &CommandClient) {
        let set_state_res = match res {
            Ok(true) => command_client.set_command_state_success(command_id).await,
            Ok(false) => Ok(()),
            Err(e) => {
                tracing::error!("Command {} failed: {:?}", command_id, e);
                command_client.set_command_state_error(command_id, &e.to_string()).await
            }
        };

        if let Err(e) = set_state_res {
            tracing::error!("Error setting command state for {}: {}", command_id, e);
        }
    }
}
