//! LinkPulz domain core — types and the backlink state machine.
//!
//! This crate has **no I/O**: no sqlx, no reqwest, no axum, no clock. Every
//! decision is a pure function of its arguments (ADR-0001, docs/08). That is
//! what lets the state machine be exhaustively tested with no database and no
//! network, and it is why the product's central promise —
//!
//!   > a failed request does not mean a lost backlink (§24, §57.10)
//!
//! — is enforced here as a property of the types, not as a convention.

pub mod anchors;
pub mod attention;
pub mod csv_export;
pub mod csv_import;
pub mod event;
pub mod forums;
pub mod impact;
pub mod keywords;
pub mod monitoring;
pub mod net;
pub mod observation;
pub mod outreach;
pub mod outreach_schedule;
pub mod reclamation;
pub mod scoring;
pub mod spot_score;
pub mod spots;
pub mod state_machine;
pub mod status;
pub mod url_normalize;
pub mod warranty;

pub use anchors::{
    AnchorKind, BrandTokens, CategorySlice, Distribution, TopAnchor, classify as classify_anchor,
    distribution as anchor_distribution,
};
pub use attention::{
    AttentionKind, Priority, priority as attention_priority, sort_key as attention_sort_key,
};
pub use event::{BacklinkEvent, EventType, change_alert_label};
pub use forums::{ForumKind, forum_kind, is_forum};
pub use impact::{Confidence, SignalState};
pub use keywords::{KeywordCandidate, candidates_from_h1s, clean_keyword};
pub use monitoring::{
    MonitoringDecision, MonitoringInputs, MonitoringLevel, MonitoringReason,
    decide as monitoring_decide, frequency_days,
};
pub use net::{
    SsrfBlock, c_class, classify_host, classify_ip, classify_port, classify_scheme, is_cdn_ip,
};
pub use observation::{BlockReason, ConnErrorKind, Detected, Observation};
pub use outreach::{
    OutreachContext, OutreachDraft, ReclaimPitch, render_draft as outreach_render_draft,
};
pub use outreach_schedule::{DispatchAction, decide as outreach_decide};
pub use reclamation::ReclaimAngle;
pub use scoring::{ProspectScore, ProspectSignals, score_prospect};
pub use spot_score::{SpotScore, SpotSignals, score as spot_score};
pub use spots::{AggregatedSpot, SerpHit, aggregate_spots, is_blocked_domain, site_host};
pub use state_machine::{BacklinkState, DetectedFields, Transition, TransitionPolicy, transition};
pub use status::{BacklinkStatus, RelSet, RelType};
pub use url_normalize::{NormalizeOptions, host_of, normalize};
pub use warranty::{
    WarrantyStatus, WarrantyType, expiration as warranty_expiration, status as warranty_status,
};
