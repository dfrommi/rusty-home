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
                .max_by_key(|exec| (exec.timestamp, exec.id.value()))
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

    #[cfg(test)]
    mod tests {
        use super::*;
        use crate::{
            core::time::DateTime,
            trigger::{OnOffDevice, UserTrigger},
        };
        use anyhow::Context as _;

        #[tokio::test]
        async fn replay_picks_the_higher_id_on_ties_and_hides_future_lifecycle_dates() -> anyhow::Result<()> {
            let timestamp = DateTime::from_static_iso("2026-03-10T12:00:00Z");
            let active_from = timestamp + t!(1 minutes);
            let active_until = timestamp + t!(2 minutes);
            let newest = UserTriggerExecution {
                id: 2.into(),
                trigger: UserTrigger::DevicePower {
                    device: OnOffDevice::InfraredHeater,
                    on: false,
                },
                timestamp,
                active_from: Some(active_from),
                active_until: Some(active_until),
                correlation_id: None,
            };
            let older = UserTriggerExecution {
                id: 1.into(),
                ..newest.clone()
            };
            let target = newest.target();
            let id = newest.id.clone();
            // SQLite returns the higher ID first for equal timestamps.
            let provider = PreloadedUserTriggerProvider {
                triggers: Arc::new(vec![newest, older]),
            };

            for (now, expected_from, expected_until) in [
                (timestamp, None, None),
                (active_from, Some(active_from), None),
                (active_until, Some(active_from), Some(active_until)),
            ] {
                let trigger = now
                    .eval_timeshifted(async { provider.get(&target) })
                    .await
                    .context("Trigger should be visible in replay")?;
                assert_eq!(trigger.id, id);
                assert_eq!(trigger.active_from, expected_from);
                assert_eq!(trigger.active_until, expected_until);
            }
            Ok(())
        }
    }
}
