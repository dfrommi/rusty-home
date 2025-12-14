use crate::{
    core::{
        HomeApi,
        time::{DateTimeRange, Duration},
    },
    device_state::DeviceStateClient,
    t,
};

mod context;
mod snapshot;

pub use context::DerivedStateProvider;
pub use context::StateCalculationContext;
pub use context::calculate_new_snapshot;
pub use snapshot::StateSnapshot;

pub async fn bootstrap_snapshot(
    duration: Duration,
    api: &HomeApi,
    device_state: &DeviceStateClient,
) -> anyhow::Result<StateSnapshot> {
    let range = DateTimeRange::new(t!(now) - duration.clone(), t!(now));
    let mut current = StateSnapshot::default();

    for dt in range.step_by(t!(30 seconds)) {
        current = dt
            .eval_timeshifted(async {
                context::calculate_new_snapshot(duration.clone(), &current, api, device_state).await
            })
            .await?;
    }

    Ok(current)
}
