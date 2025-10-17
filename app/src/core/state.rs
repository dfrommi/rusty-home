use std::collections::HashMap;

use tokio::sync::broadcast::{Receiver, Sender};

use crate::{
    Infrastructure,
    core::{HomeApi, app_event::StateChangedEvent, timeseries::DataPoint},
    home::state::{HomeState, HomeStateValue},
    port::DataPointAccess as _,
};

pub struct HomeStateEventEmitter {
    api: HomeApi,
    last_values: HashMap<HomeState, DataPoint<HomeStateValue>>,
    state_changed_rx: Receiver<StateChangedEvent>,
    home_state_updated_tx: Sender<DataPoint<HomeStateValue>>,
    home_state_changed_tx: Sender<DataPoint<HomeStateValue>>,
}

impl HomeStateEventEmitter {
    pub fn new(infrastructure: &Infrastructure) -> Self {
        Self {
            api: infrastructure.api.clone(),
            last_values: HashMap::new(),
            state_changed_rx: infrastructure.event_listener.new_state_changed_listener(),
            home_state_updated_tx: tokio::sync::broadcast::channel(256).0,
            home_state_changed_tx: tokio::sync::broadcast::channel(256).0,
        }
    }

    pub fn subscribe_updated(&self) -> Receiver<DataPoint<HomeStateValue>> {
        self.home_state_updated_tx.subscribe()
    }

    pub fn subscribe_changed(&self) -> Receiver<DataPoint<HomeStateValue>> {
        self.home_state_changed_tx.subscribe()
    }

    pub async fn run(&mut self) {
        let mut timer = tokio::time::interval(std::time::Duration::from_secs(30));

        loop {
            tokio::select! {
                _ = self.state_changed_rx.recv() => {},
                _ = timer.tick() => {},
            }

            for state in HomeState::variants() {
                if let Ok(data_point) = state.current_data_point(&self.api).await {
                    if let Err(e) = self.home_state_updated_tx.send(data_point.clone()) {
                        tracing::error!("Error sending home state updated event: {:?}", e);
                    }

                    let is_different = match self.last_values.get(&state) {
                        Some(last) => last.value != data_point.value,
                        None => true,
                    };

                    if is_different {
                        self.last_values.insert(state, data_point.clone());
                        if let Err(e) = self.home_state_changed_tx.send(data_point) {
                            tracing::error!("Error sending home state changed event: {:?}", e);
                        }
                    }
                }
            }
        }
    }
}
