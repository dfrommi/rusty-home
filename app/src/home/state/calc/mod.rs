use crate::{
    core::{
        HomeApi,
        time::{DateTimeRange, Duration},
    },
    t,
};

mod context;
mod snapshot;

pub use context::DerivedStateProvider;
pub use context::StateCalculationContext;
pub use context::calculate_new_snapshot;
pub use snapshot::StateSnapshot;

pub async fn bootstrap_snapshot(duration: Duration, api: &HomeApi) -> anyhow::Result<StateSnapshot> {
    let range = DateTimeRange::new(t!(now) - duration.clone(), t!(now));
    let mut current = StateSnapshot::default();

    for dt in range.step_by(t!(30 seconds)) {
        let current_range = DateTimeRange::new(dt - duration.clone(), dt);
        current = dt
            .eval_timeshifted(async { context::calculate_new_snapshot(current_range, current, api).await })
            .await?;
    }

    Ok(current)
}
