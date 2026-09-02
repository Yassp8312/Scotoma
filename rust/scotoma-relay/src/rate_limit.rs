use std::time::Instant;

/// Small in-memory token bucket with deterministic time injection for tests.
#[derive(Debug, Clone)]
pub(crate) struct RateLimiter {
    refill_per_second: f64,
    capacity: f64,
    tokens: f64,
    last_refill: Instant,
}

impl RateLimiter {
    pub fn new(refill_per_second: u32, capacity: u32, now: Instant) -> Self {
        Self {
            refill_per_second: f64::from(refill_per_second),
            capacity: f64::from(capacity),
            tokens: f64::from(capacity),
            last_refill: now,
        }
    }

    pub fn allow(&mut self, now: Instant) -> bool {
        if self.refill_per_second == 0.0 || self.capacity == 0.0 {
            return false;
        }
        let elapsed = now.saturating_duration_since(self.last_refill);
        self.tokens =
            (self.tokens + elapsed.as_secs_f64() * self.refill_per_second).min(self.capacity);
        self.last_refill = now;
        if self.tokens < 1.0 {
            return false;
        }
        self.tokens -= 1.0;
        true
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use super::RateLimiter;

    #[test]
    fn burst_is_bounded_and_tokens_refill_over_time() {
        let start = Instant::now();
        let mut limiter = RateLimiter::new(2, 2, start);

        assert!(limiter.allow(start));
        assert!(limiter.allow(start));
        assert!(!limiter.allow(start));
        assert!(limiter.allow(start + Duration::from_millis(500)));
        assert!(!limiter.allow(start + Duration::from_millis(500)));
        assert!(limiter.allow(start + Duration::from_secs(2)));
        assert!(limiter.allow(start + Duration::from_secs(2)));
        assert!(!limiter.allow(start + Duration::from_secs(2)));
    }

    #[test]
    fn zero_rate_or_capacity_rejects_every_request() {
        let now = Instant::now();

        assert!(!RateLimiter::new(0, 1, now).allow(now));
        assert!(!RateLimiter::new(1, 0, now).allow(now));
    }
}
