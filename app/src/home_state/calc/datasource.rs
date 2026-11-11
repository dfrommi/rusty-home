pub use device_state::{CurrentDeviceStateProvider, DeviceStateProvider, PreloadedDeviceStateProvider};

pub use user_trigger::{CurrentUserTriggerProvider, PreloadedUserTriggerProvider, UserTriggerProvider};

mod device_state {
    use std::{collections::HashMap, sync::Arc};

    use crate::{
        core::{
            time::DateTimeRange,
            timeseries::{DataFrame, DataPoint},
        },
        device_state::{DeviceStateClient, DeviceStateId, DeviceStateValue},
        t,
    };

    pub trait DeviceStateProvider {
        fn get(&self, id: &DeviceStateId) -> Option<DataPoint<DeviceStateValue>>;

        fn get_all(&self) -> HashMap<DeviceStateId, DataPoint<DeviceStateValue>> {
            DeviceStateId::variants()
                .iter()
                .filter_map(|id| self.get(id).map(|dp| (*id, dp)))
                .collect()
        }
    }

    pub struct CurrentDeviceStateProvider {
        device_states: HashMap<DeviceStateId, DataPoint<DeviceStateValue>>,
    }

    impl CurrentDeviceStateProvider {
        #[tracing::instrument(skip_all, "load_current_device_states")]
        pub async fn new(device_state_client: &DeviceStateClient) -> anyhow::Result<Self> {
            let device_states = device_state_client.get_current_for_all().await?;
            Ok(Self { device_states })
        }
    }

    impl DeviceStateProvider for CurrentDeviceStateProvider {
        fn get(&self, id: &DeviceStateId) -> Option<DataPoint<DeviceStateValue>> {
            self.device_states.get(id).cloned()
        }
    }

    #[derive(Clone)]
    pub struct PreloadedDeviceStateProvider {
        device_states: Arc<HashMap<DeviceStateId, DataFrame<DeviceStateValue>>>,
    }

    impl PreloadedDeviceStateProvider {
        pub async fn new(device_client: &DeviceStateClient, range: DateTimeRange) -> anyhow::Result<Self> {
            let device_states = device_client.get_all_data_points_in_range(range).await?;
            Ok(Self {
                device_states: Arc::new(device_states),
            })
        }
    }

    impl DeviceStateProvider for PreloadedDeviceStateProvider {
        fn get(&self, id: &DeviceStateId) -> Option<DataPoint<DeviceStateValue>> {
            self.device_states
                .get(id)
                .and_then(|df| df.prev_or_at(t!(now)).cloned())
        }
    }
}

mod user_trigger {
    use std::{collections::HashMap, sync::Arc};

    use crate::{
        core::time::DateTimeRange,
        t,
        trigger::{TriggerClient, UserTriggerExecution, UserTriggerTarget},
    };

    pub trait UserTriggerProvider {
        fn get(&self, target: &UserTriggerTarget) -> Option<UserTriggerExecution>;

        fn get_all(&self) -> HashMap<UserTriggerTarget, UserTriggerExecution> {
            UserTriggerTarget::variants()
                .iter()
                .filter_map(|target| self.get(target).map(|exec| (target.clone(), exec)))
                .collect()
        }
    }

    pub struct CurrentUserTriggerProvider {
        triggers: Vec<UserTriggerExecution>,
    }

    impl CurrentUserTriggerProvider {
        #[tracing::instrument(skip_all, "load_current_user_triggers")]
        pub async fn new(trigger_client: &TriggerClient) -> anyhow::Result<Self> {
            let triggers = trigger_client.get_latest_triggers().await?;
            Ok(Self { triggers })
        }
    }

    impl UserTriggerProvider for CurrentUserTriggerProvider {
        fn get(&self, target: &UserTriggerTarget) -> Option<UserTriggerExecution> {
            self.triggers.iter().find(|exec| exec.target() == *target).cloned()
        }
    }

    #[derive(Clone)]
    pub struct PreloadedUserTriggerProvider {
        triggers: Arc<Vec<UserTriggerExecution>>,
    }

    impl PreloadedUserTriggerProvider {
        pub async fn new(trigger_client: &TriggerClient, range: DateTimeRange) -> anyhow::Result<Self> {
            let triggers = trigger_client.get_all_triggers_active_anytime_in_range(range).await?;

            Ok(Self {
                triggers: Arc::new(triggers),
            })
        }
    }

    //Not very efficient, but amount of triggers is very small anyway
    impl UserTriggerProvider for PreloadedUserTriggerProvider {
        fn get(&self, target: &UserTriggerTarget) -> Option<UserTriggerExecution> {
            let now = t!(now);
            let mut trigger = self
                .triggers
                .iter()
                .filter(|exec| exec.target() == *target && exec.timestamp <= now)
                .max_by_key(|exec| exec.timestamp) //latest within timeshifted range
                .cloned()?;

            //Fake state at `now` timestamp
            if let Some(active_from) = trigger.active_from
                && active_from > now
            {
                trigger.active_from = None;
            }

            if let Some(active_until) = trigger.active_until
                && active_until > now
            {
                trigger.active_until = None;
            }

            Some(trigger)
        }
    }
}
