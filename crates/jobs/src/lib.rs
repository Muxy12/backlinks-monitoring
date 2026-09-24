//! Job model (docs/07). This increment is the pure core: job kinds and their
//! priorities, the retry/backoff curve, and idempotency-key construction. The
//! durable queue itself — atomic claim with `FOR UPDATE SKIP LOCKED`,
//! completion, and stuck-job reaping — wires in with `db` (it is all SQL,
//! docs/07 §2–§3).

pub mod backoff;
pub mod idempotency;
pub mod kind;

pub use backoff::{BackoffPolicy, RetryDecision, retry_delay};
pub use idempotency::check_now_key;
pub use kind::JobKind;
