pub use device_state::{CurrentDeviceStateProvider, PreloadedDeviceStateProvider};

pub use user_trigger::{CurrentUserTriggerProvider, PreloadedUserTriggerProvider};

mod device_state {
    use std::{collections::HashMap, sync::Arc};

    use crate::{
        core::timeseries::{DataFrame, DataPoint},
        device_state::{DeviceStateClient, DeviceStateId, DeviceStateValue},
        home_state::calc::context::DeviceStateProvider,
        t,
    };

    pub struct CurrentDeviceStateProvider {
        device_states: HashMap<DeviceStateId, DataPoint<DeviceStateValue>>,
    }

    impl CurrentDeviceStateProvider {
        fn new(device_states: HashMap<DeviceStateId, DataPoint<DeviceStateValue>>) -> Self {
            Self { device_states }
        }

        #[tracing::instrument(skip_all, "load_current_device_states")]
        pub async fn load(device_state_client: &DeviceStateClient) -> anyhow::Result<Self> {
            let device_states = device_state_client.get_current_for_all().await?;
            Ok(Self::new(device_states))
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
        pub fn new(device_states: HashMap<DeviceStateId, DataFrame<DeviceStateValue>>) -> Self {
            Self {
                device_states: Arc::new(device_states),
            }
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
        home_state::calc::context::UserTriggerProvider,
        trigger::{TriggerClient, UserTriggerExecution, UserTriggerTarget},
    };

    pub struct CurrentUserTriggerProvider {
        active_triggers: HashMap<UserTriggerTarget, UserTriggerExecution>,
    }

    impl CurrentUserTriggerProvider {
        //TODO filter for inactive triggers
        fn new(active_triggers: Vec<UserTriggerExecution>) -> Self {
            let mut trigger_map = HashMap::new();

            for trigger in active_triggers {
                trigger_map.insert(trigger.target(), trigger);
            }

            Self {
                active_triggers: trigger_map,
            }
        }

        #[tracing::instrument(skip_all, "load_current_user_triggers")]
        pub async fn load(trigger_client: &TriggerClient) -> anyhow::Result<Self> {
            let active_triggers = trigger_client.get_all_active_triggers().await?;
            Ok(Self::new(active_triggers))
        }
    }

    impl UserTriggerProvider for CurrentUserTriggerProvider {
        fn get(&self, target: &UserTriggerTarget) -> Option<UserTriggerExecution> {
            self.active_triggers.get(target).cloned()
        }

        fn get_all(&self) -> HashMap<UserTriggerTarget, UserTriggerExecution> {
            self.active_triggers.clone()
        }
    }

    #[derive(Clone)]
    pub struct PreloadedUserTriggerProvider {
        trigger_executions: Arc<HashMap<UserTriggerTarget, Vec<UserTriggerExecution>>>,
    }

    impl PreloadedUserTriggerProvider {
        pub fn new(trigger_executions: HashMap<UserTriggerTarget, Vec<UserTriggerExecution>>) -> Self {
            Self {
                trigger_executions: Arc::new(trigger_executions),
            }
        }
    }

    //Not very efficient, but amount of triggers is very small anyway
    impl UserTriggerProvider for PreloadedUserTriggerProvider {
        fn get(&self, target: &UserTriggerTarget) -> Option<UserTriggerExecution> {
            self.trigger_executions
                .get(target)?
                .iter()
                .filter(|exec| exec.is_active())
                .max_by_key(|exec| exec.timestamp)
                .cloned()
        }

        fn get_all(&self) -> HashMap<UserTriggerTarget, UserTriggerExecution> {
            let mut result = HashMap::new();

            for target in self.trigger_executions.keys() {
                if let Some(exec) = self.get(target) {
                    result.insert(target.clone(), exec);
                }
            }

            result
        }
    }
}
