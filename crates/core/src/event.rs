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
    SponsoredAdded,
    SponsoredRemoved,
    UgcAdded,
    UgcRemoved,
    LinkMoved,
    SourceRedirected,
    Source404,
    Source404To200,
    SourceError,
    TargetRedirected,
    Target404,
    TargetError,
    NoindexDetected,
    CanonicalChanged,
}

impl EventType {
    /// The snake_case token stored in the `event_type` Postgres enum.
    pub fn as_str(self) -> &'static str {
        match self {
            EventType::BacklinkFound => "backlink_found",
            EventType::BacklinkLost => "backlink_lost",
            EventType::BacklinkRestored => "backlink_restored",
            EventType::AnchorChanged => "anchor_changed",
            EventType::TargetChanged => "target_changed",
            EventType::DofollowToNofollow => "dofollow_to_nofollow",
            EventType::NofollowToDofollow => "nofollow_to_dofollow",
            EventType::RelChanged => "rel_changed",
            EventType::SponsoredAdded => "sponsored_added",
            EventType::SponsoredRemoved => "sponsored_removed",
            EventType::UgcAdded => "ugc_added",
            EventType::UgcRemoved => "ugc_removed",
            EventType::LinkMoved => "link_moved",
            EventType::SourceRedirected => "source_redirected",
            EventType::Source404 => "source_404",
            EventType::Source404To200 => "source_404_to_200",
            EventType::SourceError => "source_error",
            EventType::TargetRedirected => "target_redirected",
            EventType::Target404 => "target_404",
            EventType::TargetError => "target_error",
            EventType::NoindexDetected => "noindex_detected",
            EventType::CanonicalChanged => "canonical_changed",
        }
    }
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

/// Human label for a backlink-change event worth alerting the owner about, or
/// `None` for events that are NOT turned into an alert — `backlink_found` (not a
/// loss/change), `link_moved`/`canonical_changed`/`rel_changed` (low signal;
/// dofollow/nofollow transitions carry the meaningful rel change), and the
/// `competitor_*` family (handled by their own in-app alert). Single source of
/// truth for the notification pass in db::repositories::notifications.
pub fn change_alert_label(event_type: &str) -> Option<&'static str> {
    Some(match event_type {
        "backlink_lost" => "Backlink lost",
        "backlink_restored" => "Backlink restored",
        "dofollow_to_nofollow" => "Link became nofollow",
        "source_redirected" => "Source page redirected",
        "target_redirected" => "Target URL redirected",
        "source_error" => "Source page error",
        "target_error" => "Target URL error",
        "anchor_changed" => "Anchor changed",
        "target_changed" => "Target URL changed",
        "noindex_detected" => "Source page noindexed",
        _ => return None,
    })
}

#[cfg(test)]
mod change_alert_tests {
    use super::change_alert_label;

    #[test]
    fn notifies_significant_changes_only() {
        for t in [
            "backlink_lost",
            "backlink_restored",
            "dofollow_to_nofollow",
            "source_error",
            "target_error",
            "anchor_changed",
            "noindex_detected",
        ] {
            assert!(change_alert_label(t).is_some(), "{t} should alert");
        }
        // Not alerted: not a loss/change, low signal, or competitor-owned.
        for t in [
            "backlink_found",
            "link_moved",
            "canonical_changed",
            "rel_changed",
            "competitor_new_backlink",
            "competitor_new_referring_domain",
        ] {
            assert!(change_alert_label(t).is_none(), "{t} should NOT alert");
        }
    }
}
