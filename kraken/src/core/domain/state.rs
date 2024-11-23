use super::port::{StateCollector, StateStorage};

//TODO more generic with Vec of StateCollector, but that's not possible yet because of async fn on
//trait prevents building an object. async-trait crate might help.
pub async fn collect_states(
    state_storage: &impl StateStorage,
    state_collector: &mut impl StateCollector,
) -> anyhow::Result<()> {
    tracing::info!("Start persisting current states");

    for dp in state_collector.get_current_state().await? {
        state_storage.add_state(&dp.value, &dp.timestamp).await?;
    }

    loop {
        match state_collector.recv().await {
            Ok(dp) => {
                state_storage.add_state(&dp.value, &dp.timestamp).await?;
            }
            Err(e) => {
                tracing::error!("Error processing state: {:?}", e);
            }
        }
    }
}
