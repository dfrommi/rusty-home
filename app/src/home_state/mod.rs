mod calc;
mod items;

pub use calc::StateSnapshot;
pub use items::*;

use tokio::sync::broadcast::{Receiver, Sender};

use crate::core::time::Duration;
use crate::core::timeseries::DataPoint;
use crate::device_state::DeviceStateClient;
use crate::device_state::DeviceStateEvent;
use crate::home_state::calc::bootstrap_snapshot;
use crate::home_state::calc::calculate_new_snapshot;
use crate::trigger::TriggerClient;
use crate::trigger::TriggerEvent;

pub struct HomeStateRunner {
    duration: Duration,
    device_state: DeviceStateClient,
    snapshot: StateSnapshot,
    trigger_client: TriggerClient,
    state_changed_rx: Receiver<DeviceStateEvent>,
    user_trigger_rx: Receiver<TriggerEvent>,
    home_state_updated_tx: Sender<DataPoint<HomeStateValue>>,
    home_state_changed_tx: Sender<DataPoint<HomeStateValue>>,
    snapshot_updated_tx: Sender<StateSnapshot>,
}

impl HomeStateRunner {
    pub fn new(
        duration: Duration,
        rx_state: Receiver<DeviceStateEvent>,
        rx_trigger: Receiver<TriggerEvent>,
        trigger_client: TriggerClient,
        device_state: DeviceStateClient,
    ) -> Self {
        Self {
            duration,
            device_state,
            snapshot: StateSnapshot::default(),
            trigger_client,
            state_changed_rx: rx_state,
            user_trigger_rx: rx_trigger,
            home_state_updated_tx: tokio::sync::broadcast::channel(256).0,
            home_state_changed_tx: tokio::sync::broadcast::channel(256).0,
            snapshot_updated_tx: tokio::sync::broadcast::channel(64).0,
        }
    }

    pub async fn bootstrap_snapshot(&mut self) -> anyhow::Result<()> {
        self.snapshot = bootstrap_snapshot(self.duration.clone(), &self.trigger_client, &self.device_state).await?;
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
        use tokio::time::{self, Duration, Instant};

        let scheduled_duration = Duration::from_secs(30);
        let debounce_duration = Duration::from_millis(50);
        let debounce_sleeper = time::sleep(scheduled_duration);
        tokio::pin!(debounce_sleeper);

        loop {
            tokio::select! {
                //Collect state changes as many might come in short intervals
                Ok(DeviceStateEvent::Updated(_)) = self.state_changed_rx.recv() => {
                    // Put debounce timer into the near future, expiration triggers calculation
                    debounce_sleeper.as_mut().reset(Instant::now() + debounce_duration);
                },

                Ok(TriggerEvent::TriggerAdded) = self.user_trigger_rx.recv() => {
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
