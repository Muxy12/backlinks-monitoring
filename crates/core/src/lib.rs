//! Backlio domain core — types and the backlink state machine.
//!
//! This crate has **no I/O**: no sqlx, no reqwest, no axum, no clock. Every
//! decision is a pure function of its arguments (ADR-0001, docs/08). That is
//! what lets the state machine be exhaustively tested with no database and no
//! network, and it is why the product's central promise —
//!
//!   > a failed request does not mean a lost backlink (§24, §57.10)
//!
//! — is enforced here as a property of the types, not as a convention.

pub mod event;
pub mod observation;
pub mod state_machine;
pub mod status;

pub use event::{BacklinkEvent, EventType};
pub use observation::{BlockReason, ConnErrorKind, Detected, Observation};
pub use state_machine::{BacklinkState, DetectedFields, Transition, TransitionPolicy, transition};
pub use status::{BacklinkStatus, RelSet, RelType};
