use std::collections::HashMap;

use crate::{
    core::time::{DateTimeRange, Duration},
    device_state::DeviceStateClient,
    t,
    trigger::{TriggerClient, UserTriggerExecution},
};

mod context;
mod datasource;
mod snapshot;

pub use context::DerivedStateProvider;
pub use context::StateCalculationContext;
pub use datasource::CurrentDeviceStateProvider;
pub use datasource::CurrentUserTriggerProvider;
pub use snapshot::StateSnapshot;

pub async fn bootstrap_context(
    duration: Duration,
    trigger_client: &TriggerClient,
    device_state: &DeviceStateClient,
) -> anyhow::Result<StateCalculationContext> {
    let range = DateTimeRange::of_last(duration.clone());
    let mut current = None;

    let device_state_data = device_state.get_all_data_points_in_range(range.clone()).await?;
    let trigger_data: Vec<UserTriggerExecution> = trigger_client
        .get_all_triggers_active_anytime_in_range(range.clone())
        .await?;
    let trigger_map = trigger_data.into_iter().fold(HashMap::new(), |mut acc, trigger| {
        acc.entry(trigger.target()).or_insert_with(Vec::new).push(trigger);
        acc
    });

    let device_state_ds = datasource::PreloadedDeviceStateProvider::new(device_state_data);
    let trigger_ds = datasource::PreloadedUserTriggerProvider::new(trigger_map);

    for dt in range.step_by(t!(30 seconds)) {
        //Timeshift and eager load is important to get the state at the expected point in time.
        //Internal timeshift would require async
        current = dt
            .eval_timeshifted(async {
                tracing::debug!("Bootstrapping context for {}", dt);
                let new_ctx = StateCalculationContext::new(
                    device_state_ds.clone(),
                    trigger_ds.clone(),
                    current,
                    duration.clone(),
                );
                new_ctx.load_all();
                Some(new_ctx)
            })
            .await;
    }

    current.ok_or_else(|| anyhow::anyhow!("Failed to bootstrap context"))
}
