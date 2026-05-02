use infrastructure::EventListener;

use crate::{
    command::{Command, CommandEvent, CommandExecution, EnergySavingDevice},
    core::timeseries::DataPoint,
    device_state::{DeviceStateValue, EnergySaving, adapter::IncomingData, adapter::IncomingDataSource},
    t,
};

pub struct InternalDataSource {
    rx: EventListener<CommandEvent>,
}

impl InternalDataSource {
    pub fn new(rx: EventListener<CommandEvent>) -> Self {
        Self { rx }
    }
}

impl IncomingDataSource<CommandEvent, ()> for InternalDataSource {
    fn ds_name(&self) -> &str {
        "InternalDS"
    }

    async fn recv(&mut self) -> Option<CommandEvent> {
        self.rx.recv().await
    }

    fn device_id(&self, msg: &CommandEvent) -> Option<String> {
        match msg {
            CommandEvent::CommandExecuted(cmd_exec) => Some(cmd_exec.id.to_string()),
        }
    }

    fn get_channels(&self, _: &str) -> &[()] {
        &[()]
    }

    async fn to_incoming_data(&self, _: &str, _: &(), msg: &CommandEvent) -> anyhow::Result<Vec<IncomingData>> {
        let res = match msg {
            CommandEvent::CommandExecuted(CommandExecution {
                command: Command::SetEnergySaving { device, on },
                ..
            }) => {
                let dp = DataPoint::new(
                    DeviceStateValue::EnergySaving(
                        match device {
                            EnergySavingDevice::LivingRoomTv => EnergySaving::LivingRoomTv,
                        },
                        *on,
                    ),
                    t!(now),
                );
                vec![IncomingData::StateValue(dp)]
            }
            CommandEvent::CommandExecuted(_) => Vec::new(),
        };

        tracing::debug!("InternalDataSource produced incoming data: {:?}", res);

        Ok(res)
    }
}
