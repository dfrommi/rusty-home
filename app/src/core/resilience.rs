use crate::core::time::{DateTime, Duration};

#[derive(Debug, Clone)]
pub struct ExponentialBackoff {
    attempts: u32,
    base_delay: Duration,
    max_delay: Duration,
    next_retry_at: Option<DateTime>,
}

impl ExponentialBackoff {
    pub fn new(base_delay: Duration, max_delay: Duration) -> Self {
        Self {
            attempts: 0,
            base_delay,
            max_delay,
            next_retry_at: None,
        }
    }

    pub fn attempts(&self) -> u32 {
        self.attempts
    }

    pub fn reset(&mut self) {
        self.attempts = 0;
        self.next_retry_at = None;
    }

    pub fn may_retry(&self) -> bool {
        if let Some(next_retry_at) = self.next_retry_at {
            DateTime::now() >= next_retry_at
        } else {
            true
        }
    }

    pub fn bump(&mut self) {
        self.attempts = self.attempts.saturating_add(1);
        let delay = self.current_delay();
        self.next_retry_at = Some(DateTime::now() + delay);
    }

    fn current_delay(&self) -> Duration {
        let base = self.base_delay.as_secs();
        let multiplier = 2i64.saturating_pow(self.attempts.min(31));
        let delay = base.saturating_mul(multiplier).min(self.max_delay.as_secs());
        Duration::seconds(delay)
    }
}
