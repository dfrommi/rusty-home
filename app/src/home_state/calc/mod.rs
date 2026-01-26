use crate::{
    core::time::{DateTimeRange, Duration},
    device_state::DeviceStateClient,
    trigger::TriggerClient,
};

mod context;
mod datasource;
mod iter;
mod snapshot;

pub use context::DerivedStateProvider;
pub use context::StateCalculationContext;
pub use context::StateCalculationResult;
pub use datasource::CurrentDeviceStateProvider;
pub use datasource::CurrentUserTriggerProvider;
pub use iter::StateSnapshotIterator;
pub use snapshot::StateSnapshot;

pub async fn bootstrap_context(
    duration: Duration,
    device_state: DeviceStateClient,
    trigger_client: TriggerClient,
) -> anyhow::Result<StateCalculationResult> {
    let range = DateTimeRange::of_last(duration.clone());

    let mut it = iter::StateCalculationResultIterator::new(range, duration, device_state, trigger_client, false);

    while let Some(ctx) = it.next().await? {
        tracing::trace!("Bootstrapping context for {}", ctx.timestamp());
    }

    it.take().ok_or_else(|| anyhow::anyhow!("Failed to bootstrap context"))
}
