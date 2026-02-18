use std::time::Duration;

#[derive(Clone, Debug)]
pub struct BackoffPolicy {
    pub initial_ms: u64,
    pub max_ms: u64,
    pub factor: f64,
    pub jitter: f64,
}

impl BackoffPolicy {
    /// Default backoff for network errors: 1s → 2s → 4s → 8s (max 60s)
    pub fn default_network() -> Self {
        Self {
            initial_ms: 1000,   // 1 second
            max_ms: 60000,      // 1 minute
            factor: 2.0,        // Double each time
            jitter: 0.1,        // 10% randomness
        }
    }

    /// Longer backoff for rate limiting: 5s → 10s → 20s (max 5min)
    pub fn rate_limit() -> Self {
        Self {
            initial_ms: 5000,   // 5 seconds
            max_ms: 300000,     // 5 minutes
            factor: 2.0,
            jitter: 0.2,        // 20% randomness
        }
    }

    /// Compute backoff duration for a given attempt number (1-indexed)
    pub fn compute(&self, attempt: u32) -> Duration {
        use rand::Rng;

        // Calculate base delay with exponential backoff
        let base = self.initial_ms as f64
            * self.factor.powi(attempt.saturating_sub(1) as i32);

        // Add random jitter to avoid thundering herd
        let jitter = base * self.jitter * rand::thread_rng().gen::<f64>();

        // Cap at max_ms
        let total_ms = (base + jitter).min(self.max_ms as f64);

        Duration::from_millis(total_ms as u64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backoff_increases() {
        let policy = BackoffPolicy::default_network();

        let delay1 = policy.compute(1);
        let delay2 = policy.compute(2);
        let delay3 = policy.compute(3);

        // Each delay should be roughly double (accounting for jitter)
        // delay1 ≈ 1000ms, delay2 ≈ 2000ms, delay3 ≈ 4000ms
        assert!(delay2 > delay1, "delay2 ({:?}) should be > delay1 ({:?})", delay2, delay1);
        assert!(delay3 > delay2, "delay3 ({:?}) should be > delay2 ({:?})", delay3, delay2);
    }

    #[test]
    fn test_backoff_caps_at_max() {
        let policy = BackoffPolicy {
            initial_ms: 1000,
            max_ms: 5000,
            factor: 2.0,
            jitter: 0.0, // No jitter for predictable test
        };

        // After many attempts, should cap at max_ms
        let delay = policy.compute(10);
        assert_eq!(delay, Duration::from_millis(5000));
    }

    #[test]
    fn test_first_attempt_is_initial() {
        let policy = BackoffPolicy {
            initial_ms: 1000,
            max_ms: 60000,
            factor: 2.0,
            jitter: 0.0, // No jitter for predictable test
        };

        let delay = policy.compute(1);
        assert_eq!(delay, Duration::from_millis(1000));
    }

    #[test]
    fn test_rate_limit_policy() {
        let policy = BackoffPolicy::rate_limit();

        let delay1 = policy.compute(1);

        // First attempt should be around 5 seconds (with jitter up to 20%)
        assert!(delay1 >= Duration::from_millis(5000));
        assert!(delay1 <= Duration::from_millis(6000)); // 5000 + 20% jitter
    }
}
