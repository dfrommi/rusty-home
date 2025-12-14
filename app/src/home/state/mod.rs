mod calc;
mod items;

pub use calc::StateSnapshot;
pub use items::*;

use calc::DerivedStateProvider;
use calc::StateCalculationContext;
use tokio::sync::broadcast::{Receiver, Sender};

use crate::core::HomeApi;
use crate::core::app_event::StateChangedEvent;
use crate::core::time::DateTimeRange;
use crate::core::time::Duration;
use crate::core::timeseries::DataPoint;
use crate::home::state::calc::bootstrap_snapshot;
use crate::home::state::calc::calculate_new_snapshot;

pub struct HomeStateRunner {
    duration: Duration,
    api: HomeApi,
    snapshot: StateSnapshot,
    //TODO make a persistent state change event
    state_changed_rx: Receiver<StateChangedEvent>,
    home_state_updated_tx: Sender<DataPoint<HomeStateValue>>,
    home_state_changed_tx: Sender<DataPoint<HomeStateValue>>,
    snapshot_updated_tx: Sender<StateSnapshot>,
}

impl HomeStateRunner {
    pub fn new(duration: Duration, rx: Receiver<StateChangedEvent>, api: HomeApi) -> Self {
        Self {
            duration,
            api,
            snapshot: StateSnapshot::default(),
            state_changed_rx: rx,
            home_state_updated_tx: tokio::sync::broadcast::channel(256).0,
            home_state_changed_tx: tokio::sync::broadcast::channel(256).0,
            snapshot_updated_tx: tokio::sync::broadcast::channel(64).0,
        }
    }

    pub async fn bootstrap_snapshot(&mut self) -> anyhow::Result<()> {
        self.snapshot = bootstrap_snapshot(self.duration.clone(), &self.api).await?;
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
                _ = timer.tick() => {},
            }

            let range = DateTimeRange::of_last(self.duration.clone());

            let new_snapshot = match calculate_new_snapshot(range, self.snapshot.clone(), &self.api).await {
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

                    let is_different = match self.snapshot.get(state) {
                        Some(last) => last.value != data_point.value,
                        None => true,
                    };

                    if is_different {
                        if let Err(e) = self.home_state_changed_tx.send(data_point) {
                            tracing::error!("Error sending home state changed event: {:?}", e);
                        }
                    }
                }
            }
        }
    }
}
