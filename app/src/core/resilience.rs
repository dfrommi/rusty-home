use crate::core::time::Duration;

#[derive(Debug, Clone)]
pub struct ExponentialBackoff {
    attempts: u32,
    base_delay: Duration,
    max_delay: Duration,
}

impl ExponentialBackoff {
    pub fn new(base_delay: Duration, max_delay: Duration) -> Self {
        Self {
            attempts: 0,
            base_delay,
            max_delay,
        }
    }

    pub fn reset(&mut self) {
        self.attempts = 0;
    }

    pub fn next_delay(&self) -> Duration {
        let base = self.base_delay.as_secs();
        let multiplier = 2i64.saturating_pow(self.attempts.min(31));
        let delay = base.saturating_mul(multiplier).min(self.max_delay.as_secs());
        Duration::seconds(delay)
    }

    pub fn bump(&mut self) {
        self.attempts = self.attempts.saturating_add(1);
    }

    pub fn attempts(&self) -> u32 {
        self.attempts
    }
}
