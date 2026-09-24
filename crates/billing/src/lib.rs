//! Billing: the plan catalog and entitlement rules (§39). This increment is
//! pure — no Stripe, no database — so plan limits and quota decisions are
//! unit-tested in isolation. Stripe Checkout/Portal/webhooks and persistence
//! wire in on top (they pull `async-stripe` and `db`).
//!
//! Plan configuration is centralized here (§39 "keep plan configuration
//! centralized"): one place defines price, backlink limit, project limit and
//! features, and everything else reads it.

pub mod entitlements;
pub mod plans;
pub mod webhook;

pub use entitlements::{AddOutcome, Entitlements};
pub use plans::{Plan, PlanSpec};
pub use webhook::{WebhookError, verify_signature};
