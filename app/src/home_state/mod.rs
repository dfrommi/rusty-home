mod calc;
mod items;

pub use calc::StateSnapshot;
use infrastructure::EventEmitter;
use infrastructure::{EventBus, EventListener};
pub use items::*;

use crate::core::time::Duration;
use crate::core::timeseries::DataPoint;
use crate::device_state::DeviceStateClient;
use crate::device_state::DeviceStateEvent;
use crate::home_state::calc::bootstrap_snapshot;
use crate::home_state::calc::calculate_new_snapshot;
use crate::trigger::TriggerClient;
use crate::trigger::TriggerEvent;

#[derive(Debug, Clone)]
pub enum HomeStateEvent {
    SnapshotUpdated(StateSnapshot),
    Updated(DataPoint<HomeStateValue>),
    Changed(DataPoint<HomeStateValue>),
}

pub struct HomeStateRunner {
    duration: Duration,
    device_state: DeviceStateClient,
    snapshot: StateSnapshot,
    trigger_client: TriggerClient,
    device_state_rx: EventListener<DeviceStateEvent>,
    trigger_rx: EventListener<TriggerEvent>,
    event_bus: EventBus<HomeStateEvent>,
    //TODO in service
    event_emitter: EventEmitter<HomeStateEvent>,
}

impl HomeStateRunner {
    pub fn new(
        duration: Duration,
        device_state_rx: EventListener<DeviceStateEvent>,
        trigger_rx: EventListener<TriggerEvent>,
        trigger_client: TriggerClient,
        device_state: DeviceStateClient,
    ) -> Self {
        let event_bus = EventBus::new(256);
        Self {
            duration,
            device_state,
            snapshot: StateSnapshot::default(),
            trigger_client,
            device_state_rx,
            trigger_rx,
            event_emitter: event_bus.emitter(),
            event_bus,
        }
    }

    pub async fn bootstrap_snapshot(&mut self) -> anyhow::Result<()> {
        self.snapshot = bootstrap_snapshot(self.duration.clone(), &self.trigger_client, &self.device_state).await?;
        Ok(())
    }

    pub fn subscribe(&self) -> EventListener<HomeStateEvent> {
        self.event_bus.subscribe()
    }

    pub async fn run(mut self) {
        use tokio::time::{self, Duration, Instant};

        let scheduled_duration = Duration::from_secs(30);
        let debounce_duration = Duration::from_millis(50);
        let debounce_sleeper = time::sleep(scheduled_duration);
        tokio::pin!(debounce_sleeper);

        loop {
            tokio::select! {
                //Collect state changes as many might come in short intervals
                event = self.device_state_rx.recv() => if let Some(DeviceStateEvent::Updated(_)) = event {
                    // Put debounce timer into the near future, expiration triggers calculation
                    debounce_sleeper.as_mut().reset(Instant::now() + debounce_duration);
                },

                event = self.trigger_rx.recv() => if let Some(TriggerEvent::TriggerAdded) = event {
                    self.update_snapshot().await;

                    //Schedule next regular update
                    debounce_sleeper.as_mut().reset(Instant::now() + scheduled_duration);
                },

                // Debounce elapsed
                () = &mut debounce_sleeper => {
                    self.update_snapshot().await;
                    //Schedule next regular update
                    debounce_sleeper.as_mut().reset(Instant::now() + scheduled_duration);
                }
            }
        }
    }

    async fn update_snapshot(&mut self) {
        let old_snapshot = self.snapshot.clone();
        let new_snapshot = match calculate_new_snapshot(
            self.duration.clone(),
            &old_snapshot,
            &self.device_state,
            &self.trigger_client,
        )
        .await
        {
            Ok(snapshot) => snapshot,
            Err(e) => {
                tracing::error!("Error calculating new home state snapshot: {:?}", e);
                return;
            }
        };

        self.event_emitter
            .send(HomeStateEvent::SnapshotUpdated(new_snapshot.clone()));

        for state in HomeStateId::variants() {
            if let Some(data_point) = new_snapshot.get(state) {
                self.event_emitter.send(HomeStateEvent::Updated(data_point.clone()));

                let is_different = match old_snapshot.get(state) {
                    Some(previous) => previous.value != data_point.value,
                    None => true,
                };

                if is_different {
                    self.event_emitter.send(HomeStateEvent::Changed(data_point));
                }
            }
        }

        self.snapshot = new_snapshot;
    }
}
