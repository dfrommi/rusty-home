use std::fmt::Display;

use anyhow::Context;
use chrono::Timelike;

use super::DateTime;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Time {
    pub(super) delegate: chrono::NaiveTime,
}

impl Time {
    pub(super) fn new(delegate: chrono::NaiveTime) -> Self {
        Self { delegate }
    }

    pub fn today(&self) -> DateTime {
        DateTime::now().at(*self).unwrap()
    }

    pub fn yesterday(&self) -> DateTime {
        DateTime::now().at(*self).unwrap().on_prev_day()
    }

    pub fn at(hour: u32, minute: u32) -> anyhow::Result<Self> {
        Ok(Self {
            delegate: chrono::NaiveTime::from_hms_opt(hour, minute, 0)
                .context(format!("Error parsing time {}:{}", hour, minute))?,
        })
    }

    pub fn hour(&self) -> u32 {
        self.delegate.hour()
    }

    pub fn minute(&self) -> u32 {
        self.delegate.minute()
    }
}

impl Display for Time {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.delegate)
    }
}
