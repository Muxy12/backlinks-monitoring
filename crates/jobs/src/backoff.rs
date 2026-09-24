//! Retry backoff and the retry/kill decision (docs/07 §3).
//!
//! On failure a job is retried with exponential backoff plus **full jitter**,
//! until `max_attempts` is exhausted (→ dead) or the error is permanent. Jitter
//! is essential: without it, one Neon blip fails N jobs simultaneously and they
//! all retry simultaneously, reproducing the outage (docs/07 §3).
//!
//! The jitter sample is an argument, so the curve is a pure function and the
//! tests are deterministic; the worker passes a real random `[0,1)` sample.

use chrono::Duration;

#[derive(Debug, Clone, Copy)]
pub struct BackoffPolicy {
    pub base: Duration,
    pub factor: u32,
    pub cap: Duration,
}

impl Default for BackoffPolicy {
    fn default() -> Self {
        // base 2s, doubling, capped at 1h (docs/07 §3).
        BackoffPolicy {
            base: Duration::seconds(2),
            factor: 2,
            cap: Duration::hours(1),
        }
    }
}

/// What to do with a failed job.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetryDecision {
    /// Reschedule after `delay`.
    Retry { delay: Duration },
    /// Give up: attempts exhausted or a permanent error.
    Dead,
}

/// Full-jitter exponential backoff for attempt `n` (1-based): the base delay is
/// `min(base * factor^(n-1), cap)`, then jittered to a random point in
/// `[0, base_delay]`. `jitter` is a sample in `[0, 1)`.
pub fn backoff_delay(policy: &BackoffPolicy, attempt: i32, jitter: f64) -> Duration {
    let exp = attempt.saturating_sub(1).clamp(0, 30) as u32;
    let mult = policy.factor.saturating_pow(exp) as i64;
    let raw = policy.base.num_seconds().saturating_mul(mult);
    let capped = raw.min(policy.cap.num_seconds()).max(0);
    let jittered = (capped as f64 * jitter.clamp(0.0, 1.0)) as i64;
    Duration::seconds(jittered)
}

/// Decide whether to retry a failed job. `permanent` is the handler's verdict
/// that retrying cannot help (e.g. a malformed payload, docs/07 §3).
pub fn retry_delay(
    policy: &BackoffPolicy,
    attempts: i32,
    max_attempts: i32,
    permanent: bool,
    jitter: f64,
) -> RetryDecision {
    if permanent || attempts >= max_attempts {
        RetryDecision::Dead
    } else {
        RetryDecision::Retry {
            delay: backoff_delay(policy, attempts, jitter),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn policy() -> BackoffPolicy {
        BackoffPolicy::default()
    }

    #[test]
    fn full_jitter_zero_is_zero_and_one_is_the_cap_curve() {
        // attempt 1: base_delay = 2s. jitter 0 -> 0s, jitter ~1 -> ~2s.
        assert_eq!(backoff_delay(&policy(), 1, 0.0), Duration::seconds(0));
        assert_eq!(backoff_delay(&policy(), 1, 1.0), Duration::seconds(2));
    }

    #[test]
    fn backoff_grows_exponentially_until_capped() {
        // With jitter 1.0 (upper edge): 2,4,8,16,... capped at 3600.
        let p = policy();
        assert_eq!(backoff_delay(&p, 1, 1.0), Duration::seconds(2));
        assert_eq!(backoff_delay(&p, 2, 1.0), Duration::seconds(4));
        assert_eq!(backoff_delay(&p, 3, 1.0), Duration::seconds(8));
        assert_eq!(backoff_delay(&p, 5, 1.0), Duration::seconds(32));
        // Far out, the cap holds.
        assert_eq!(backoff_delay(&p, 20, 1.0), Duration::hours(1));
    }

    #[test]
    fn jitter_stays_within_the_base_delay() {
        let p = policy();
        for &j in &[0.1, 0.37, 0.5, 0.99] {
            let d = backoff_delay(&p, 4, j); // base_delay = 16s
            assert!(
                d >= Duration::seconds(0) && d <= Duration::seconds(16),
                "jitter {j} in range"
            );
        }
    }

    #[test]
    fn permanent_error_is_dead_immediately() {
        assert_eq!(retry_delay(&policy(), 1, 5, true, 0.5), RetryDecision::Dead);
    }

    #[test]
    fn exhausted_attempts_are_dead() {
        assert_eq!(
            retry_delay(&policy(), 5, 5, false, 0.5),
            RetryDecision::Dead
        );
        assert_eq!(
            retry_delay(&policy(), 6, 5, false, 0.5),
            RetryDecision::Dead
        );
    }

    #[test]
    fn transient_within_budget_retries() {
        match retry_delay(&policy(), 2, 5, false, 1.0) {
            RetryDecision::Retry { delay } => assert_eq!(delay, Duration::seconds(4)),
            RetryDecision::Dead => panic!("should retry"),
        }
    }

    #[test]
    fn no_overflow_at_extreme_attempts() {
        // Must not panic or wrap.
        let _ = backoff_delay(&policy(), i32::MAX, 1.0);
    }
}
