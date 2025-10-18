pub mod energy_meter;
pub mod grafana;
pub mod homeassistant;
pub mod homekit;
pub mod mcp;
pub mod metrics_export;
pub mod tasmota;
pub mod z2m;

pub use command::CommandExecutorRunner;
pub use incoming::IncomingDataSourceRunner;

mod incoming {
    use crate::{
        core::{HomeApi, timeseries::DataPoint},
        home::{availability::ItemAvailability, state::PersistentHomeStateValue, trigger::UserTrigger},
    };

    #[derive(Debug, Clone, derive_more::From)]
    pub enum IncomingData {
        StateValue(DataPoint<PersistentHomeStateValue>),
        UserTrigger(UserTrigger),
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
        api: HomeApi,
        _marker: std::marker::PhantomData<(M, C)>,
    }

    impl<M, C, S> IncomingDataSourceRunner<M, C, S>
    where
        M: std::fmt::Debug,
        C: std::fmt::Debug,
        S: IncomingDataSource<M, C>,
    {
        pub fn new(source: S, api: HomeApi) -> Self {
            Self {
                source,
                api,
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
                        if let Err(e) = self.api.add_state(&dp.value, &dp.timestamp).await {
                            tracing::error!("Error processing state {:?}: {:?}", dp, e);
                        }
                    }

                    IncomingData::UserTrigger(trigger) => {
                        if let Err(e) = self.api.add_user_trigger(trigger.clone()).await {
                            tracing::error!("Error processing user trigger {:?}: {:?}", trigger, e);
                        }
                    }

                    IncomingData::ItemAvailability(item) => {
                        if let Err(e) = self.api.add_item_availability(item.clone()).await {
                            tracing::error!("Error processing item availability {:?}: {:?}", item, e);
                        }
                    }
                }
            }
        }
    }
}

mod command {
    use infrastructure::TraceContext;

    use crate::{
        core::HomeApi,
        home::command::{Command, CommandExecution},
    };

    pub trait CommandExecutor {
        //Returns true if command was executed
        async fn execute_command(&self, command: &Command) -> anyhow::Result<bool>;
    }

    pub struct CommandExecutorRunner<E: CommandExecutor> {
        executor: E,
        pending_command_rx: tokio::sync::broadcast::Receiver<CommandExecution>,
        api: HomeApi,
    }

    impl<E: CommandExecutor> CommandExecutorRunner<E> {
        pub fn new(
            executor: E,
            pending_command_rx: tokio::sync::broadcast::Receiver<CommandExecution>,
            api: HomeApi,
        ) -> Self {
            Self {
                executor,
                pending_command_rx,
                api,
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

                process_command(msg, &self.api, &self.executor).await;
            }
        }
    }

    #[tracing::instrument(skip_all, fields(command = ?cmd.command))]
    async fn process_command(cmd: CommandExecution, api: &HomeApi, executor: &impl CommandExecutor) {
        TraceContext::continue_from(&cmd.correlation_id);

        let res = executor.execute_command(&cmd.command).await;

        handle_execution_result(cmd.id, res, api).await;
    }

    async fn handle_execution_result(command_id: i64, res: anyhow::Result<bool>, api: &HomeApi) {
        let set_state_res = match res {
            Ok(true) => api.set_command_state_success(command_id).await,
            Ok(false) => Ok(()),
            Err(e) => {
                tracing::error!("Command {} failed: {:?}", command_id, e);
                api.set_command_state_error(command_id, &e.to_string()).await
            }
        };

        if let Err(e) = set_state_res {
            tracing::error!("Error setting command state for {}: {}", command_id, e);
        }
    }
}
