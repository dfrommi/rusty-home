use r#macro::{EnumVariants, Id};

use crate::{
    core::time::{DateTime, Duration},
    t,
};

#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq, Id, EnumVariants)]
pub enum CompletionTime {
    WashingMachineEstimated,
}

impl CompletionTime {
    pub fn minimum_storage_delta(&self, value: DateTime) -> Option<Duration> {
        let now = t!(now);

        if now > value {
            return None;
        }

        let diff = value.elapsed_since(now);

        let duration = if diff > t!(30 minutes) {
            t!(2 minutes)
        } else if diff > t!(10 minutes) {
            t!(1 minutes)
        } else if diff > t!(5 minutes) {
            t!(30 seconds)
        } else {
            return None;
        };

        Some(duration)
    }
}
