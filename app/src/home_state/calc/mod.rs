use std::collections::HashMap;

use rayon::prelude::*;

use crate::{
    core::{
        time::{DateTime, DateTimeRange, Duration},
        timeseries::DataFrame,
    },
    device_state::DeviceStateClient,
    home_state::{HomeStateId, HomeStateValue},
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
    // FIXED_NOW is a Tokio task-local and is not inherited by Rayon workers.
    calculate_next_result_with_parallelism(
        keep,
        current,
        device_state_client,
        trigger_client,
        parallel_calculation_enabled(),
    )
}

fn parallel_calculation_enabled() -> bool {
    !DateTime::is_shifted()
}

fn calculate_next_result_with_parallelism(
    keep: Duration,
    current: StateCalculationResult,
    device_state_client: DeviceStateClient,
    trigger_client: TriggerClient,
    parallel: bool,
) -> StateCalculationResult {
    let start_time = t!(now);
    let cutoff = start_time - keep.clone();
    let ctx = StateCalculationContext::new(device_state_client, trigger_client, current);
    let ids = HomeStateId::variants();
    let data = if parallel {
        let _span = tracing::trace_span!("collect_home_state_data", state_count = ids.len()).entered();
        calculate_state_data(&ctx, &ids, cutoff, true)
    } else {
        calculate_state_data(&ctx, &ids, cutoff, false)
    };

    StateCalculationResult::new(start_time, data)
}

fn calculate_state_data(
    ctx: &StateCalculationContext,
    ids: &[HomeStateId],
    cutoff: DateTime,
    parallel: bool,
) -> HashMap<HomeStateId, DataFrame<HomeStateValue>> {
    let calculate = |id: &HomeStateId| match ctx.all_since(*id, cutoff) {
        Some(df) if !df.is_empty() => Some((*id, df)),
        _ => {
            tracing::warn!("Home state value for {:?} could not be calculated", id);
            None
        }
    };

    if parallel {
        ids.par_iter().filter_map(calculate).collect()
    } else {
        ids.iter().filter_map(calculate).collect()
    }
}
