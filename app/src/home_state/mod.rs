mod calc;
mod items;

use std::collections::HashMap;

use anyhow::Context;

pub use calc::{StateCalculationResult, StateSnapshot};
use infrastructure::EventEmitter;
use infrastructure::{EventBus, EventListener};
pub use items::*;

use crate::core::time::Duration;
use crate::core::timeseries::DataPoint;
use crate::device_state::DeviceStateClient;
use crate::device_state::DeviceStateEvent;
use crate::home_state::calc::{bootstrap_context, calculate_next_result};
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

    pub async fn bootstrap(&self) -> anyhow::Result<StateCalculationResult> {
        tracing::info!("Starting bootstrap of home state context");
        let state_result =
            bootstrap_context(self.duration.clone(), self.device_state.clone(), self.trigger_client.clone())
                .await
                .context("Failed to bootstrap home state context")?;

        tracing::info!("Calculating initial home state context");
        Ok(self.update_context(state_result))
    }

    pub async fn run(mut self, mut state_result: StateCalculationResult) {
        use tokio::time::{self, Duration, Instant};

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
                    state_result = self.update_context(state_result);
                    //Schedule next regular update
                    debounce_sleeper.as_mut().reset(Instant::now() + scheduled_duration);
                },

                // Debounce elapsed
                () = &mut debounce_sleeper => {
                    state_result = self.update_context(state_result);
                    //Schedule next regular update
                    debounce_sleeper.as_mut().reset(Instant::now() + scheduled_duration);
                }
            }
        }
    }

    #[tracing::instrument(skip_all)]
    fn update_context(&self, old_result: StateCalculationResult) -> StateCalculationResult {
        tracing::trace!("Updating home state context");

        let old_dps: HashMap<HomeStateId, DataPoint<HomeStateValue>> = HomeStateId::variants()
            .into_iter()
            .filter_map(|id| old_result.get_latest(id).map(|dp| (id, dp.clone())))
            .collect();

        let duration = self.duration.clone();
        let device_state = self.device_state.clone();
        let trigger_client = self.trigger_client.clone();
        let new_result = calculate_next_result(duration, old_result, device_state, trigger_client);

        self.emit_events(&new_result, old_dps);

        tracing::trace!("Completed update of home state context");

        new_result
    }

    #[tracing::instrument(skip_all)]
    fn emit_events(
        &self,
        new_result: &StateCalculationResult,
        latest_previous_dps: HashMap<HomeStateId, DataPoint<HomeStateValue>>,
    ) {
        let new_snapshot = StateSnapshot::new(new_result.clone());

        for state in HomeStateId::variants() {
            if let Some(data_point) = new_snapshot.get(state) {
                self.event_emitter.send(HomeStateEvent::Updated(data_point.clone()));

                let is_different = match latest_previous_dps.get(&state) {
                    Some(previous) => previous.value != data_point.value,
                    None => true,
                };

                if is_different {
                    self.event_emitter.send(HomeStateEvent::Changed(data_point));
                }
            }
        }

        self.event_emitter.send(HomeStateEvent::SnapshotUpdated(new_snapshot));
    }
}
