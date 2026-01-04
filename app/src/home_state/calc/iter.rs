use std::collections::HashMap;

use crate::{
    core::time::{DateTime, DateTimeRange, Duration},
    device_state::DeviceStateClient,
    home_state::{
        StateSnapshot,
        calc::{
            StateCalculationContext,
            datasource::{PreloadedDeviceStateProvider, PreloadedUserTriggerProvider},
        },
    },
    t,
    trigger::{TriggerClient, UserTriggerExecution},
};

pub struct StateCalculationContextIterator {
    full_range: DateTimeRange,
    device_client: DeviceStateClient,
    trigger_client: TriggerClient,
    keep_duration: Duration,

    current: Option<StateCalculationContext>,

    preload_range: DateTimeRange,
    device_data_source: Option<PreloadedDeviceStateProvider>,
    trigger_data_source: Option<PreloadedUserTriggerProvider>,
}

impl StateCalculationContextIterator {
    pub fn new(
        full_range: DateTimeRange,
        keep_duration: Duration,
        device_client: DeviceStateClient,
        trigger_client: TriggerClient,
    ) -> Self {
        Self {
            preload_range: Self::preload_range_starting_at(*full_range.start(), &full_range),
            device_data_source: None,
            trigger_data_source: None,
            full_range,
            device_client,
            trigger_client,
            keep_duration,
            current: None,
        }
    }

    pub fn take(mut self) -> Option<StateCalculationContext> {
        self.current.take()
    }

    pub async fn next(&mut self) -> anyhow::Result<Option<&StateCalculationContext>> {
        let next_dt = match &self.current {
            Some(ctx) => ctx.timestamp() + t!(30 seconds),
            None => *self.full_range.start(),
        };

        if !self.full_range.contains(&next_dt) {
            return Ok(None);
        }

        //Reset when next block needs to be loaded
        if !self.preload_range.contains(&next_dt) {
            self.preload_range = Self::preload_range_starting_at(next_dt, &self.full_range);
            self.device_data_source = None;
            self.trigger_data_source = None;
        }

        let device_ds = match &self.device_data_source {
            Some(ds) => ds.clone(),
            None => {
                let ds = Self::create_device_ds(self.device_client.clone(), self.full_range.clone()).await?;
                self.device_data_source = Some(ds.clone());
                ds
            }
        };

        let trigger_ds = match &self.trigger_data_source {
            Some(ds) => ds.clone(),
            None => {
                let ds = Self::create_trigger_ds(self.trigger_client.clone(), self.full_range.clone()).await?;
                self.trigger_data_source = Some(ds.clone());
                ds
            }
        };

        //Timeshift and eager load is important to get the state at the expected point in time.
        //Internal timeshift would require async
        let new_ctx = next_dt
            .eval_timeshifted(async {
                let new_ctx = StateCalculationContext::new(
                    device_ds,
                    trigger_ds,
                    self.current.take(),
                    self.keep_duration.clone(),
                );
                new_ctx.load_all();
                new_ctx
            })
            .await;

        self.current = Some(new_ctx);

        Ok(self.current.as_ref())
    }

    fn preload_range_starting_at(dt: DateTime, clamp: &DateTimeRange) -> DateTimeRange {
        DateTimeRange::new(dt, (dt + t!(30 days)).min(*clamp.end()))
    }

    async fn create_device_ds(
        device_client: DeviceStateClient,
        range: DateTimeRange,
    ) -> anyhow::Result<PreloadedDeviceStateProvider> {
        let device_state_data = device_client.get_all_data_points_in_range(range).await?;
        Ok(PreloadedDeviceStateProvider::new(device_state_data))
    }

    async fn create_trigger_ds(
        trigger_client: TriggerClient,
        range: DateTimeRange,
    ) -> anyhow::Result<PreloadedUserTriggerProvider> {
        let trigger_data: Vec<UserTriggerExecution> =
            trigger_client.get_all_triggers_active_anytime_in_range(range).await?;
        let trigger_map = trigger_data.into_iter().fold(HashMap::new(), |mut acc, trigger| {
            acc.entry(trigger.target()).or_insert_with(Vec::new).push(trigger);
            acc
        });
        Ok(PreloadedUserTriggerProvider::new(trigger_map))
    }
}

pub struct StateSnapshotIterator {
    inner: StateCalculationContextIterator,
}

impl StateSnapshotIterator {
    pub fn new(
        full_range: DateTimeRange,
        keep_duration: Duration,
        device_client: DeviceStateClient,
        trigger_client: TriggerClient,
    ) -> Self {
        Self {
            inner: StateCalculationContextIterator::new(full_range, keep_duration, device_client, trigger_client),
        }
    }

    pub async fn next(&mut self) -> anyhow::Result<Option<StateSnapshot>> {
        match self.inner.next().await? {
            Some(ctx) => Ok(Some(ctx.as_snapshot())),
            None => Ok(None),
        }
    }
}
