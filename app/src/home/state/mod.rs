mod calc;
mod items;

pub use calc::StateSnapshot;
pub use items::*;

use calc::StateCalculationContext;
use tokio::sync::broadcast::{Receiver, Sender};

use crate::core::HomeApi;
use crate::core::app_event::StateChangedEvent;
use crate::core::app_event::UserTriggerEvent;
use crate::core::time::Duration;
use crate::core::timeseries::DataPoint;
use crate::device_state::DeviceStateClient;
use crate::home::state::calc::bootstrap_snapshot;
use crate::home::state::calc::calculate_new_snapshot;

pub struct HomeStateRunner {
    duration: Duration,
    api: HomeApi,
    device_state: DeviceStateClient,
    snapshot: StateSnapshot,
    //TODO make a persistent state change event
    state_changed_rx: Receiver<StateChangedEvent>,
    user_trigger_rx: Receiver<UserTriggerEvent>,
    home_state_updated_tx: Sender<DataPoint<HomeStateValue>>,
    home_state_changed_tx: Sender<DataPoint<HomeStateValue>>,
    snapshot_updated_tx: Sender<StateSnapshot>,
}

impl HomeStateRunner {
    pub fn new(
        duration: Duration,
        rx_state: Receiver<StateChangedEvent>,
        rx_trigger: Receiver<UserTriggerEvent>,
        api: HomeApi,
        device_state: DeviceStateClient,
    ) -> Self {
        Self {
            duration,
            api,
            device_state,
            snapshot: StateSnapshot::default(),
            state_changed_rx: rx_state,
            user_trigger_rx: rx_trigger,
            home_state_updated_tx: tokio::sync::broadcast::channel(256).0,
            home_state_changed_tx: tokio::sync::broadcast::channel(256).0,
            snapshot_updated_tx: tokio::sync::broadcast::channel(64).0,
        }
    }

    pub async fn bootstrap_snapshot(&mut self) -> anyhow::Result<()> {
        self.snapshot = bootstrap_snapshot(self.duration.clone(), &self.api, &self.device_state).await?;
        Ok(())
    }

    pub fn subscribe_state_updated(&self) -> Receiver<DataPoint<HomeStateValue>> {
        self.home_state_updated_tx.subscribe()
    }

    pub fn subscribe_state_changed(&self) -> Receiver<DataPoint<HomeStateValue>> {
        self.home_state_changed_tx.subscribe()
    }

    pub fn subscribe_snapshot_updated(&self) -> Receiver<StateSnapshot> {
        self.snapshot_updated_tx.subscribe()
    }

    pub async fn run(mut self) {
        let mut timer = tokio::time::interval(std::time::Duration::from_secs(30));

        loop {
            tokio::select! {
                _ = self.state_changed_rx.recv() => {},
                _ = self.user_trigger_rx.recv() => {},
                _ = timer.tick() => {},
            }

            let old_snapshot = self.snapshot.clone();
            let new_snapshot =
                match calculate_new_snapshot(self.duration.clone(), &old_snapshot, &self.api, &self.device_state).await
                {
                    Ok(snapshot) => snapshot,
                    Err(e) => {
                        tracing::error!("Error calculating new home state snapshot: {:?}", e);
                        continue;
                    }
                };

            if let Err(e) = self.snapshot_updated_tx.send(new_snapshot.clone()) {
                tracing::error!("Error sending snapshot updated event: {}", e);
            }

            for state in HomeState::variants() {
                if let Some(data_point) = new_snapshot.get(state.clone()) {
                    if let Err(e) = self.home_state_updated_tx.send(data_point.clone()) {
                        tracing::error!("Error sending home state updated event: {:?}", e);
                    }

                    let is_different = match old_snapshot.get(state.clone()) {
                        Some(previous) => previous.value != data_point.value,
                        None => true,
                    };

                    if is_different {
                        if let Err(e) = self.home_state_changed_tx.send(data_point) {
                            tracing::error!("Error sending home state changed event: {:?}", e);
                        }
                    }
                }
            }

            self.snapshot = new_snapshot;
        }
    }
}
