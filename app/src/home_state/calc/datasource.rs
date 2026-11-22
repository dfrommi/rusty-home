pub use user_trigger::{CurrentUserTriggerProvider, PreloadedUserTriggerProvider, UserTriggerProvider};

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
