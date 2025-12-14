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
pub use context::calculate_new_snapshot;
pub use snapshot::StateSnapshot;

pub async fn bootstrap_snapshot(
    duration: Duration,
    trigger_client: &TriggerClient,
    device_state: &DeviceStateClient,
) -> anyhow::Result<StateSnapshot> {
    let range = DateTimeRange::new(t!(now) - duration.clone(), t!(now));
    let mut current = StateSnapshot::default();

    for dt in range.step_by(t!(30 seconds)) {
        current = dt
            .eval_timeshifted(async {
                context::calculate_new_snapshot(duration.clone(), &current, device_state, &trigger_client).await
            })
            .await?;
    }

    Ok(current)
}
