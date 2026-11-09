use std::collections::HashMap;

use crate::{
    core::time::{DateTimeRange, Duration},
    device_state::DeviceStateClient,
    home_state::HomeStateId,
    t,
    trigger::TriggerClient,
};

mod context;
mod datasource;
mod iter;
mod snapshot;

pub use context::DerivedStateProvider;
pub use context::DeviceStateProvider;
pub use context::StateCalculationContext;
pub use context::StateCalculationResult;
pub use context::UserTriggerProvider;
pub use datasource::CurrentDeviceStateProvider;
pub use datasource::CurrentUserTriggerProvider;

pub use snapshot::StateSnapshot;

pub async fn bootstrap_context(
    duration: Duration,
    device_state: DeviceStateClient,
    trigger_client: TriggerClient,
) -> anyhow::Result<StateCalculationResult> {
    let range = DateTimeRange::of_last(duration.clone());

    let mut it = iter::StateCalculationResultIterator::new(range, duration, device_state, trigger_client);

    while let Some(ctx) = it.next().await? {
        tracing::trace!("Bootstrapping context for {}", ctx.timestamp());
    }

    it.take().ok_or_else(|| anyhow::anyhow!("Failed to bootstrap context"))
}

pub fn calculate_next_result(
    keep: Duration,
    current: StateCalculationResult,
    device_state_provider: &impl DeviceStateProvider,
    user_trigger_provider: &impl UserTriggerProvider,
) -> StateCalculationResult {
    let start_time = t!(now);
    let cutoff = start_time - keep.clone();

    let ctx = StateCalculationContext::new(device_state_provider.get_all(), user_trigger_provider.get_all(), current);

    let mut data = HashMap::new();

    for id in HomeStateId::variants().iter() {
        match ctx.all_since(*id, cutoff) {
            Some(df) if !df.is_empty() => {
                data.insert(*id, df);
            }
            _ => {
                tracing::warn!("Home state value for {:?} could not be calculated", id);
                continue;
            }
        };
    }

    StateCalculationResult::new(start_time, data)
}
