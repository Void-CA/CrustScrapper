use std::time::Duration;

use tokio::sync::Mutex;
use tokio::time::{Instant, sleep};

pub struct RateLimiter {
    min_interval: Duration,
    max_interval: Duration,
    interval: Mutex<Duration>,
    last: Mutex<Instant>,
}

impl RateLimiter {
    pub fn new(requests_per_second: f64) -> Self {
        let rps = requests_per_second.clamp(0.1, 1000.0);
        let min_interval = Duration::from_secs_f64(1.0 / rps);
        let now = Instant::now();
        Self {
            min_interval,
            max_interval: Duration::from_secs(30),
            interval: Mutex::new(min_interval),
            last: Mutex::new(now.checked_sub(min_interval).unwrap_or(now)),
        }
    }

    pub async fn acquire(&self) {
        let interval = *self.interval.lock().await;
        let mut last = self.last.lock().await;
        let now = Instant::now();
        let next = *last + interval;
        if next > now {
            sleep(next - now).await;
        }
        *last = Instant::now();
    }

    pub async fn on_throttle(&self) {
        let mut interval = self.interval.lock().await;
        *interval = (*interval * 2).min(self.max_interval);
    }

    pub async fn on_success(&self) {
        let mut interval = self.interval.lock().await;
        if *interval > self.min_interval {
            let reduced = *interval * 95 / 100;
            *interval = reduced.max(self.min_interval);
        }
    }

    pub async fn current_rps(&self) -> f64 {
        let interval = *self.interval.lock().await;
        1.0 / interval.as_secs_f64().max(f64::EPSILON)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn throttling_increases_interval_and_success_recovers() {
        let limiter = RateLimiter::new(10.0);
        let base = limiter.current_rps().await;
        limiter.on_throttle().await;
        assert!(limiter.current_rps().await < base);
        for _ in 0..200 {
            limiter.on_success().await;
        }
        assert!((limiter.current_rps().await - base).abs() < 1.0);
    }
}
