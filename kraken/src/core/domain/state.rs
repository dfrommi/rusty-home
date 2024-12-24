use tokio::sync::mpsc;

use crate::core::IncomingData;

use super::{port::StateStorage, UserTriggerStorage};

pub async fn collect_states(
    mut incoming_data: mpsc::Receiver<IncomingData>,
    storage: &(impl StateStorage + UserTriggerStorage),
) -> anyhow::Result<()> {
    tracing::info!("Start persisting current states");

    loop {
        let data = incoming_data.recv().await;
        match &data {
            Some(IncomingData::StateValue(dp)) => {
                if let Err(e) = storage.add_state(&dp.value, &dp.timestamp).await {
                    tracing::error!("Error processing state {:?}: {:?}", data, e);
                }
            }

            Some(IncomingData::UserTrigger(trigger)) => {
                if let Err(e) = storage.add_user_trigger(trigger.clone()).await {
                    tracing::error!("Error processing user trigger {:?}: {:?}", trigger, e);
                }
            }

            None => {
                tracing::debug!("Event receiver closed");
            }
        }
    }
}
