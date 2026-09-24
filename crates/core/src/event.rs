//! Immutable change events (§29, docs/08 §6). Events are the record; the
//! current state on `backlinks` is a cache of the latest truth. A transition
//! emits an event **per condition that changed**, independent of which status
//! won the precedence contest.

use serde::{Deserialize, Serialize};

/// The change types we record. Mirrors the `event_type` Postgres enum
/// (migrations/0001) one-to-one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventType {
    BacklinkFound,
    BacklinkLost,
    BacklinkRestored,
    AnchorChanged,
    TargetChanged,
    DofollowToNofollow,
    NofollowToDofollow,
    RelChanged,
    SourceRedirected,
    Source404,
    SourceError,
    TargetRedirected,
    Target404,
    TargetError,
    NoindexDetected,
    CanonicalChanged,
}

/// One emitted event. `previous`/`current` are snapshots (plain strings here;
/// the persistence layer widens them to JSONB), so they stay meaningful even if
/// the backlink is later edited (docs/08 §6, the §30 "history is never
/// destroyed" rule).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BacklinkEvent {
    pub event_type: EventType,
    pub previous: Option<String>,
    pub current: Option<String>,
}

impl BacklinkEvent {
    pub fn new(event_type: EventType, previous: Option<String>, current: Option<String>) -> Self {
        BacklinkEvent {
            event_type,
            previous,
            current,
        }
    }
}
