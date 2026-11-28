use crate::{
    core::time::{DateTime, DateTimeRange, Duration},
    device_state::DeviceStateClient,
    home_state::calc::{StateCalculationResult, calculate_next_result, datasource::PreloadedUserTriggerProvider},
    t,
    trigger::TriggerClient,
};

pub struct StateCalculationResultIterator {
    full_range: DateTimeRange,
    device_client: DeviceStateClient,
    trigger_client: TriggerClient,
    keep_duration: Duration,

    current: Option<StateCalculationResult>,

    preload_range: DateTimeRange,
    trigger_data_source: Option<PreloadedUserTriggerProvider>,
}

impl StateCalculationResultIterator {
    pub fn new(
        full_range: DateTimeRange,
        keep_duration: Duration,
        device_client: DeviceStateClient,
        trigger_client: TriggerClient,
    ) -> Self {
        Self {
            preload_range: Self::preload_range_starting_at(*full_range.start(), &full_range),
            trigger_data_source: None,
            full_range,
            device_client,
            trigger_client,
            keep_duration,
            current: None,
        }
    }

    pub fn take(mut self) -> Option<StateCalculationResult> {
        self.current.take()
    }

    pub async fn next(&mut self) -> anyhow::Result<Option<&StateCalculationResult>> {
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
            self.trigger_data_source = None;
        }

        let trigger_ds = match &self.trigger_data_source {
            Some(ds) => ds.clone(),
            None => {
                let ds = PreloadedUserTriggerProvider::new(&self.trigger_client, self.full_range.clone())?;
                self.trigger_data_source = Some(ds.clone());
                ds
            }
        };

        //Timeshift and eager load is important to get the state at the expected point in time.
        //Internal timeshift would require async
        let new_res = next_dt
            .eval_timeshifted(async {
                calculate_next_result(
                    self.keep_duration.clone(),
                    self.current.take().unwrap_or_default(),
                    self.device_client.clone(),
                    &trigger_ds,
                )
            })
            .await;

        self.current = Some(new_res);

        Ok(self.current.as_ref())
    }

    fn preload_range_starting_at(dt: DateTime, clamp: &DateTimeRange) -> DateTimeRange {
        DateTimeRange::new(dt, (dt + t!(30 days)).min(*clamp.end()))
    }
}
