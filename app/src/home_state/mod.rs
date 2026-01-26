mod calc;
mod items;

pub use calc::{StateSnapshot, StateSnapshotIterator};
use infrastructure::EventEmitter;
use infrastructure::{EventBus, EventListener};
pub use items::*;

use crate::core::time::{DateTimeRange, Duration};
use crate::core::timeseries::DataPoint;
use crate::device_state::DeviceStateClient;
use crate::device_state::DeviceStateEvent;
use crate::home_state::calc::{
    CurrentDeviceStateProvider, CurrentUserTriggerProvider, StateCalculationContext, bootstrap_context,
};
use crate::trigger::TriggerClient;
use crate::trigger::TriggerEvent;

#[derive(Debug, Clone)]
pub enum HomeStateEvent {
    SnapshotUpdated(StateSnapshot),
    Updated(DataPoint<HomeStateValue>),
    Changed(DataPoint<HomeStateValue>),
}

pub struct HomeStateModule {
    duration: Duration,
    device_state: DeviceStateClient,
    trigger_client: TriggerClient,
    device_state_rx: EventListener<DeviceStateEvent>,
    trigger_rx: EventListener<TriggerEvent>,
    event_bus: EventBus<HomeStateEvent>,
    //TODO in service
    event_emitter: EventEmitter<HomeStateEvent>,
}

#[derive(Clone)]
pub struct HomeStateClient {
    keep: Duration,
    trigger_client: TriggerClient,
    device_state: DeviceStateClient,
}

impl HomeStateModule {
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
            trigger_client,
            device_state_rx,
            trigger_rx,
            event_emitter: event_bus.emitter(),
            event_bus,
        }
    }

    pub fn subscribe(&self) -> EventListener<HomeStateEvent> {
        self.event_bus.subscribe()
    }

    pub fn client(&self) -> HomeStateClient {
        HomeStateClient {
            keep: self.duration.clone(),
            trigger_client: self.trigger_client.clone(),
            device_state: self.device_state.clone(),
        }
    }

    pub async fn run(mut self) {
        use tokio::time::{self, Duration, Instant};

        tracing::info!("Starting bootstrap of home state context");
        let mut context =
            bootstrap_context(self.duration.clone(), self.device_state.clone(), self.trigger_client.clone())
                .await
                .expect("Failed to bootstrap home state context");
        let mut snapshot = context.as_snapshot();

        tracing::info!("Calculating initial home state context");
        (context, snapshot) = self.update_context(context, snapshot).await;
        tracing::info!("Completed bootstrap of home state context");

        let scheduled_duration = Duration::from_secs(30);
        let debounce_duration = Duration::from_millis(50);
        let debounce_sleeper = time::sleep(scheduled_duration);
        tokio::pin!(debounce_sleeper);

        loop {
            tokio::select! {
                //Collect state changes as many might come in short intervals
                event = self.device_state_rx.recv() => if let Some(DeviceStateEvent::Changed(_)) = event {
                    // Put debounce timer into the near future, expiration triggers calculation
                    debounce_sleeper.as_mut().reset(Instant::now() + debounce_duration);
                },

                event = self.trigger_rx.recv() => if let Some(TriggerEvent::TriggerAdded) = event {
                    (context, snapshot) = self.update_context(context, snapshot).await;

                    //Schedule next regular update
                    debounce_sleeper.as_mut().reset(Instant::now() + scheduled_duration);
                },

                // Debounce elapsed
                () = &mut debounce_sleeper => {
                    (context, snapshot) = self.update_context(context, snapshot).await;
                    //Schedule next regular update
                    debounce_sleeper.as_mut().reset(Instant::now() + scheduled_duration);
                }
            }
        }
    }

    #[tracing::instrument(skip_all)]
    async fn update_context(
        &self,
        old_context: StateCalculationContext,
        old_snapshot: StateSnapshot,
    ) -> (StateCalculationContext, StateSnapshot) {
        tracing::trace!("Updating home state context");

        let device_state = CurrentDeviceStateProvider::load(&self.device_state).await;
        let trigger_state = CurrentUserTriggerProvider::load(&self.trigger_client).await;

        let new_context = match (device_state, trigger_state) {
            (Ok(ds), Ok(ts)) => StateCalculationContext::new(ds, ts, Some(old_context), self.duration.clone(), true),
            (Err(e), _) => {
                tracing::error!("Failed to load device state for home state update: {:?}", e);
                old_context
            }
            (_, Err(e)) => {
                tracing::error!("Failed to load trigger state for home state update: {:?}", e);
                old_context
            }
        };

        new_context.load_all();

        let new_snapshot = new_context.as_snapshot();

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

        tracing::trace!("Completed update of home state context");

        (new_context, new_snapshot)
    }
}

impl HomeStateClient {
    pub fn snapshot_iter(&self, range: DateTimeRange) -> StateSnapshotIterator {
        StateSnapshotIterator::new(
            range,
            self.keep.clone(),
            self.device_state.clone(),
            self.trigger_client.clone(),
            false,
        )
    }
}
