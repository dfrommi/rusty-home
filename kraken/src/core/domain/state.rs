use tokio::sync::mpsc;

use crate::core::IncomingData;

use super::port::StateStorage;

pub async fn collect_states(
    mut incoming_data: mpsc::Receiver<IncomingData>,
    state_storage: &impl StateStorage,
) -> anyhow::Result<()> {
    tracing::info!("Start persisting current states");

    loop {
        let data = incoming_data.recv().await;
        match &data {
            Some(IncomingData::StateValue(dp)) => {
                if let Err(e) = state_storage.add_state(&dp.value, &dp.timestamp).await {
                    tracing::error!("Error processing state {:?}: {:?}", data, e);
                }
            }

            None => {
                tracing::debug!("Event receiver closed");
            }
        }
    }
}
