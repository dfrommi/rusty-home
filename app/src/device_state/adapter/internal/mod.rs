use infrastructure::EventListener;

use crate::{
    command::{Command, CommandEvent, CommandExecution, EnergySavingDevice},
    core::timeseries::DataPoint,
    device_state::{
        DeviceStateValue, EnergySaving,
        adapter::{IncomingData, IncomingDataSource},
    },
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

impl IncomingDataSource for InternalDataSource {
    async fn recv_multi(&mut self) -> Option<Vec<IncomingData>> {
        loop {
            let msg = self.rx.recv().await?;
            let Some(data) = incoming_data_from_command_event(&msg) else {
                continue;
            };

            tracing::debug!("InternalDataSource produced incoming data: {:?}", data);
            return Some(data);
        }
    }
}

fn incoming_data_from_command_event(msg: &CommandEvent) -> Option<Vec<IncomingData>> {
    match msg {
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
            Some(vec![IncomingData::StateValue(dp)])
        }
        CommandEvent::CommandExecuted(_) => None,
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use infrastructure::EventBus;

    use crate::{
        command::{CommandState, PowerToggle},
        core::id::ExternalId,
    };

    use super::*;

    #[tokio::test]
    async fn skips_noop_commands_until_energy_saving_command() {
        let bus = EventBus::new(8);
        let mut ds = InternalDataSource::new(bus.subscribe());
        let emitter = bus.emitter();

        emitter.send(command_event(
            1,
            Command::SetPower {
                device: PowerToggle::Dehumidifier,
                power_on: true,
            },
        ));
        emitter.send(command_event(
            2,
            Command::SetEnergySaving {
                device: EnergySavingDevice::LivingRoomTv,
                on: true,
            },
        ));

        let updates = ds.recv_multi().await.expect("internal source closed");

        assert_eq!(updates.len(), 1);
        match &updates[0] {
            IncomingData::StateValue(dp) => {
                assert_eq!(dp.value, DeviceStateValue::EnergySaving(EnergySaving::LivingRoomTv, true))
            }
            IncomingData::ItemAvailability(_) => panic!("expected state value"),
        }
    }

    fn command_event(id: i64, command: Command) -> CommandEvent {
        CommandEvent::CommandExecuted(CommandExecution {
            id,
            command,
            state: CommandState::Success,
            created: t!(now),
            source: ExternalId::new_static("test", "test"),
            user_trigger_id: None,
            correlation_id: None,
        })
    }
}
