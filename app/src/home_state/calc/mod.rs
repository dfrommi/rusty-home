use std::collections::HashMap;

use crate::{
    core::time::{DateTimeRange, Duration},
    device_state::DeviceStateClient,
    home_state::HomeStateId,
    t,
    trigger::TriggerClient,
};

mod context;
mod snapshot;

pub use context::DerivedStateProvider;
pub use context::StateCalculationContext;
pub use context::StateCalculationResult;

pub use snapshot::StateSnapshot;

pub async fn bootstrap_context(
    duration: Duration,
    device_state: DeviceStateClient,
    trigger_client: TriggerClient,
) -> anyhow::Result<StateCalculationResult> {
    let range = DateTimeRange::of_last(duration.clone());

    let mut current = None;

    for next_dt in range.step_by(t!(30 seconds)) {
        let result = next_dt
            .eval_timeshifted(async {
                calculate_next_result(
                    duration.clone(),
                    current.take().unwrap_or_default(),
                    device_state.clone(),
                    trigger_client.clone(),
                )
            })
            .await;

        tracing::trace!("Bootstrapping context for {}", result.timestamp());
        current = Some(result);
    }

    current.ok_or_else(|| anyhow::anyhow!("Failed to bootstrap context"))
}

pub fn calculate_next_result(
    keep: Duration,
    current: StateCalculationResult,
    device_state_client: DeviceStateClient,
    trigger_client: TriggerClient,
) -> StateCalculationResult {
    let start_time = t!(now);
    let cutoff = start_time - keep.clone();

    let ctx = StateCalculationContext::new(device_state_client, trigger_client, current);

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
