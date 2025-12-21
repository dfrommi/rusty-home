use crate::{
    core::time::{DateTimeRange, Duration},
    device_state::DeviceStateClient,
    t,
    trigger::TriggerClient,
};

mod context;
mod snapshot;

pub use context::DerivedStateProvider;
pub use context::StateCalculationContext;
pub use context::create_standalone_context;
pub use snapshot::StateSnapshot;

pub async fn bootstrap_context(
    duration: Duration,
    trigger_client: &TriggerClient,
    device_state: &DeviceStateClient,
) -> anyhow::Result<StateCalculationContext> {
    let range = DateTimeRange::new(t!(now) - duration.clone(), t!(now));
    let mut current = None;

    for dt in range.step_by(t!(30 seconds)) {
        let new_ctx = dt
            .eval_timeshifted(async { create_standalone_context(device_state, trigger_client).await })
            .await?;
        current = Some(new_ctx.with_history(current, duration.clone()));
    }

    current.ok_or_else(|| anyhow::anyhow!("Failed to bootstrap context"))
}
