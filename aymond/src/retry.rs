use std::sync::Arc;
use std::time::Duration;

pub type RetryStrategy = Arc<dyn Fn(u32) -> Option<Duration> + Send + Sync>;

pub struct ExponentialBackoff {
    base_duration: Duration,
    max_retries: u32,
    jitter: f64,
}

impl ExponentialBackoff {
    pub fn new() -> Self {
        Self {
            base_duration: Duration::from_millis(20),
            max_retries: 5,
            jitter: 0.3,
        }
    }

    pub fn base_duration(mut self, d: Duration) -> Self {
        self.base_duration = d;
        self
    }

    pub fn max_retries(mut self, n: u32) -> Self {
        self.max_retries = n;
        self
    }

    pub fn jitter(mut self, j: f64) -> Self {
        self.jitter = j.clamp(0.0, 1.0);
        self
    }

    pub fn build(self) -> RetryStrategy {
        Arc::new(move |attempt: u32| {
            if attempt >= self.max_retries {
                return None;
            }
            let base_ms = self.base_duration.as_millis() as f64 * (1u64 << attempt) as f64;
            let deterministic = base_ms * (1.0 - self.jitter);
            let random = base_ms * self.jitter * fastrand::f64();
            let duration_ms = deterministic + random;
            Some(Duration::from_millis(duration_ms as u64))
        })
    }
}

impl Default for ExponentialBackoff {
    fn default() -> Self {
        Self::new()
    }
}

pub fn default_retry_strategy() -> RetryStrategy {
    ExponentialBackoff::new().build()
}
