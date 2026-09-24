//! One-shot scheduler tick, for cron-style invocation (ADR-0002 §scheduler).
//! Takes pg_advisory_xact_lock, enqueues due checks, exits. Phase 3.
fn main() {}
