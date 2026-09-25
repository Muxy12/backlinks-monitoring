//! The backlink state machine (docs/08). One pure function, `transition`.
//!
//! The rule this module exists to enforce (§24, §57.10):
//!
//!   > A failed request does not mean a lost backlink. "The link is not on the
//!   > page" and "we could not look at the page" are different facts and must
//!   > never collapse into the same state.
//!
//! Positive news is applied immediately; negative news must be confirmed by N
//! consecutive observations. That asymmetry is intentional — a false `live`
//! corrects itself next cycle, while a false `lost` costs the customer a call
//! to a publisher.

use crate::event::{BacklinkEvent, EventType};
use crate::observation::{Detected, Observation};
use crate::status::{BacklinkStatus, RelType};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

/// The detected-field snapshot carried on a backlink between checks. Events
/// compare a new detection against the **previous detection** held here — not
/// against the customer's `expected_*` — so a link imported with a wrong
/// expectation does not emit a change event on every check (docs/08 §6.1).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DetectedFields {
    pub anchor_text: Option<String>,
    pub target_url: Option<String>,
    pub rel_attributes: Vec<String>,
    pub is_dofollow: Option<bool>,
    pub canonical: Option<String>,
    pub noindex: bool,
    /// Link nature carried between checks (docs/06 §3.2).
    pub is_image: bool,
    pub image_alt: Option<String>,
}

/// The current persisted state the machine reads. It does not include the row's
/// whole record — only what a transition needs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BacklinkState {
    pub status: BacklinkStatus,
    /// The customer's expectation. Drives the `changed`/`nofollow` *status*.
    pub expected_anchor_text: Option<String>,
    pub expected_rel_type: RelType,
    /// What we last detected. Drives the *events*.
    pub detected: DetectedFields,
    pub consecutive_failure_count: i32,
    pub last_seen_at: Option<DateTime<Utc>>,
}

impl BacklinkState {
    /// A freshly created, never-checked backlink.
    pub fn pending(expected_anchor_text: Option<String>, expected_rel_type: RelType) -> Self {
        BacklinkState {
            status: BacklinkStatus::Pending,
            expected_anchor_text,
            expected_rel_type,
            detected: DetectedFields::default(),
            consecutive_failure_count: 0,
            last_seen_at: None,
        }
    }
}

/// Confirmation thresholds and scheduling curve. Config, not constants
/// (docs/08 §3), so they can be tuned without touching the logic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransitionPolicy {
    pub absent_to_lost: i32,
    pub source_gone_to_error: i32,
    pub source_http_error_to_error: i32,
    pub unreachable_to_unknown: i32,
    pub blocked_to_blocked: i32,

    /// Normal cadence for a stable `live` link.
    pub base_interval: Duration,
    /// Base of the aggressive retry curve while failures are unconfirmed.
    pub base_retry: Duration,
    /// Cap on the retry backoff.
    pub max_retry: Duration,
    /// Cadence once a link is confirmed `lost` — we keep watching, links return.
    pub lost_interval: Duration,
    /// Cadence for a link blocked by robots.txt — the answer will not change soon.
    pub blocked_interval: Duration,
}

impl Default for TransitionPolicy {
    fn default() -> Self {
        TransitionPolicy {
            absent_to_lost: 2,
            source_gone_to_error: 2,
            source_http_error_to_error: 3,
            unreachable_to_unknown: 3,
            blocked_to_blocked: 2,
            base_interval: Duration::days(7),
            base_retry: Duration::minutes(30),
            max_retry: Duration::hours(6),
            lost_interval: Duration::days(3),
            blocked_interval: Duration::days(30),
        }
    }
}

/// The result of a transition. `next_check_at` is always set here — never by a
/// cron sweep that guesses (docs/08 §7).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Transition {
    pub status: BacklinkStatus,
    pub previous_status: BacklinkStatus,
    pub consecutive_failure_count: i32,
    pub next_check_at: DateTime<Utc>,
    pub last_seen_at: Option<DateTime<Utc>>,
    pub detected: DetectedFields,
    pub events: Vec<BacklinkEvent>,
}

/// Apply one observation. Pure: `now` and `policy` are arguments; no clock, no
/// I/O. Same inputs, same output, every time.
pub fn transition(
    current: &BacklinkState,
    obs: &Observation,
    now: DateTime<Utc>,
    policy: &TransitionPolicy,
) -> Transition {
    let previous_status = current.status;
    let mut events = Vec::new();
    let mut detected = current.detected.clone();
    let mut consecutive = current.consecutive_failure_count;
    let mut last_seen = current.last_seen_at;

    let status = match obs {
        // ── Positive: applied immediately, resets the failure counter ────────
        Observation::Present(d) => {
            consecutive = 0;
            last_seen = Some(now);
            apply_present(current, d, &mut detected, &mut events)
        }

        // ── Negative: confirmed over N consecutive observations ──────────────
        Observation::Absent => {
            consecutive += 1;
            if consecutive >= policy.absent_to_lost {
                push_once(
                    &mut events,
                    previous_status,
                    BacklinkStatus::Lost,
                    EventType::BacklinkLost,
                );
                BacklinkStatus::Lost
            } else if previous_status == BacklinkStatus::Lost {
                // Already confirmed lost — stay lost, don't demote to suspected.
                BacklinkStatus::Lost
            } else {
                // Page fetched OK, link absent, but not yet confirmed (§24): a
                // distinct, honest state — no LOST event yet.
                BacklinkStatus::SuspectedLost
            }
        }

        Observation::SourceGone { status } => {
            consecutive += 1;
            if consecutive >= policy.source_gone_to_error {
                push_once(
                    &mut events,
                    previous_status,
                    BacklinkStatus::SourceError,
                    EventType::Source404,
                );
                let _ = status;
                BacklinkStatus::SourceError
            } else {
                previous_status
            }
        }

        Observation::SourceHttpError { status } => {
            consecutive += 1;
            if consecutive >= policy.source_http_error_to_error {
                push_once(
                    &mut events,
                    previous_status,
                    BacklinkStatus::SourceError,
                    EventType::SourceError,
                );
                let _ = status;
                BacklinkStatus::SourceError
            } else {
                previous_status
            }
        }

        Observation::SourceUnreachable { .. } => {
            consecutive += 1;
            if consecutive >= policy.unreachable_to_unknown {
                BacklinkStatus::Unknown
            } else {
                previous_status
            }
        }

        Observation::SourceBlocked { .. } => {
            consecutive += 1;
            if consecutive >= policy.blocked_to_blocked {
                BacklinkStatus::Blocked
            } else {
                previous_status
            }
        }

        Observation::SourceRedirected { to, permanent } => {
            // A redirect is an immediate, known fact about the source, not an
            // unconfirmed failure — but it should not clobber the counter.
            events.push(BacklinkEvent::new(
                EventType::SourceRedirected,
                detected.target_url.clone(),
                Some(to.clone()),
            ));
            let _ = permanent;
            BacklinkStatus::Redirected
        }
    };

    let next_check_at = schedule(status, consecutive, now, policy);

    Transition {
        status,
        previous_status,
        consecutive_failure_count: consecutive,
        next_check_at,
        last_seen_at: last_seen,
        detected,
        events,
    }
}

/// Handle a `Present` observation: emit change/restore events (comparing to the
/// **previous detection**), update the detected snapshot, and compute the
/// resulting status from expectations via precedence (docs/08 §2.1, §6).
fn apply_present(
    current: &BacklinkState,
    d: &Detected,
    detected: &mut DetectedFields,
    events: &mut Vec<BacklinkEvent>,
) -> BacklinkStatus {
    let prev = &current.detected;

    // First-ever detection, or recovery from a down/away state.
    match current.status {
        BacklinkStatus::Pending => {
            events.push(BacklinkEvent::new(
                EventType::BacklinkFound,
                None,
                d.target_url.clone(),
            ));
        }
        s if is_down(s) => {
            events.push(BacklinkEvent::new(
                EventType::BacklinkRestored,
                Some(status_label(s).to_string()),
                d.target_url.clone(),
            ));
        }
        _ => {}
    }

    // Anchor change — measured against the previous DETECTION, not expectation.
    if prev.anchor_text.is_some() && prev.anchor_text != d.anchor_text {
        events.push(BacklinkEvent::new(
            EventType::AnchorChanged,
            prev.anchor_text.clone(),
            d.anchor_text.clone(),
        ));
    }
    // Target change — likewise against the previous detection.
    if prev.target_url.is_some() && prev.target_url != d.target_url {
        events.push(BacklinkEvent::new(
            EventType::TargetChanged,
            prev.target_url.clone(),
            d.target_url.clone(),
        ));
    }
    // Follow-boundary crossings.
    match prev.is_dofollow {
        Some(true) if !d.is_dofollow => events.push(BacklinkEvent::new(
            EventType::DofollowToNofollow,
            Some("dofollow".into()),
            Some("nofollow".into()),
        )),
        Some(false) if d.is_dofollow => events.push(BacklinkEvent::new(
            EventType::NofollowToDofollow,
            Some("nofollow".into()),
            Some("dofollow".into()),
        )),
        _ => {}
    }
    // Granular rel transitions. dofollow<->nofollow is handled above; here we
    // emit sponsored/ugc added/removed — but only after a first detection, so a
    // freshly discovered link does not spuriously "add" its attributes.
    let new_rel = d.rel.to_sorted_vec();
    if prev.is_dofollow.is_some() {
        let had = |tok: &str| prev.rel_attributes.iter().any(|r| r == tok);
        let new_spon = d.rel.contains(RelType::Sponsored);
        let new_ugc = d.rel.contains(RelType::Ugc);
        if new_spon && !had("sponsored") {
            events.push(BacklinkEvent::new(
                EventType::SponsoredAdded,
                None,
                Some("sponsored".into()),
            ));
        } else if !new_spon && had("sponsored") {
            events.push(BacklinkEvent::new(
                EventType::SponsoredRemoved,
                Some("sponsored".into()),
                None,
            ));
        }
        if new_ugc && !had("ugc") {
            events.push(BacklinkEvent::new(
                EventType::UgcAdded,
                None,
                Some("ugc".into()),
            ));
        } else if !new_ugc && had("ugc") {
            events.push(BacklinkEvent::new(
                EventType::UgcRemoved,
                Some("ugc".into()),
                None,
            ));
        }
    }
    // Canonical / noindex page signals.
    if prev.canonical != d.canonical {
        events.push(BacklinkEvent::new(
            EventType::CanonicalChanged,
            prev.canonical.clone(),
            d.canonical.clone(),
        ));
    }
    if !prev.noindex && d.noindex {
        events.push(BacklinkEvent::new(EventType::NoindexDetected, None, None));
    }

    // Update the stored snapshot.
    detected.anchor_text = d.anchor_text.clone();
    detected.target_url = d.target_url.clone();
    detected.rel_attributes = new_rel;
    detected.is_dofollow = Some(d.is_dofollow);
    detected.canonical = d.canonical.clone();
    detected.noindex = d.noindex;
    detected.is_image = d.is_image;
    detected.image_alt = d.image_alt.clone();

    // Resulting status from expectations, by precedence.
    let mut status = BacklinkStatus::Live;

    let anchor_mismatch = match (&current.expected_anchor_text, &d.anchor_text) {
        (Some(exp), Some(got)) => normalize(exp) != normalize(got),
        (Some(_), None) => true,
        _ => false,
    };
    let target_mismatch = current
        .detected
        .target_url
        .as_ref()
        .zip(d.target_url.as_ref())
        .map(|_| false) // handled via expected below; detected drives events not status
        .unwrap_or(false);
    let _ = target_mismatch;

    if anchor_mismatch {
        status = status.max_precedence(BacklinkStatus::Changed);
    }
    if current.expected_rel_type == RelType::Dofollow && !d.is_dofollow {
        status = status.max_precedence(BacklinkStatus::Nofollow);
    }

    status
}

/// States from which a `Present` observation is a recovery.
fn is_down(s: BacklinkStatus) -> bool {
    matches!(
        s,
        BacklinkStatus::Lost
            | BacklinkStatus::SourceError
            | BacklinkStatus::TargetError
            | BacklinkStatus::Unknown
            | BacklinkStatus::Blocked
            | BacklinkStatus::Redirected
    )
}

fn status_label(s: BacklinkStatus) -> &'static str {
    match s {
        BacklinkStatus::Pending => "pending",
        BacklinkStatus::Live => "live",
        BacklinkStatus::SuspectedLost => "suspected_lost",
        BacklinkStatus::Lost => "lost",
        BacklinkStatus::Changed => "changed",
        BacklinkStatus::Nofollow => "nofollow",
        BacklinkStatus::Redirected => "redirected",
        BacklinkStatus::SourceError => "source_error",
        BacklinkStatus::TargetError => "target_error",
        BacklinkStatus::Blocked => "blocked",
        BacklinkStatus::Unknown => "unknown",
    }
}

/// Emit `event` only if the status actually crosses into `target`.
fn push_once(
    events: &mut Vec<BacklinkEvent>,
    previous: BacklinkStatus,
    target: BacklinkStatus,
    event: EventType,
) {
    if previous != target {
        events.push(BacklinkEvent::new(
            event,
            Some(status_label(previous).into()),
            Some(status_label(target).into()),
        ));
    }
}

fn normalize(s: &str) -> String {
    s.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

/// Compute `next_check_at` (docs/08 §7). Jitter is applied by the caller (the
/// scheduler) so this stays pure and deterministic for tests.
fn schedule(
    status: BacklinkStatus,
    consecutive: i32,
    now: DateTime<Utc>,
    policy: &TransitionPolicy,
) -> DateTime<Utc> {
    let delta = if consecutive > 0 && !status.is_unverifiable() && status != BacklinkStatus::Lost {
        // Unconfirmed failure: retry aggressively, capped.
        backoff(policy.base_retry, consecutive, policy.max_retry)
    } else {
        match status {
            BacklinkStatus::Lost => policy.lost_interval,
            BacklinkStatus::Blocked => policy.blocked_interval,
            BacklinkStatus::Unknown => {
                backoff(policy.base_retry, consecutive.max(1), policy.max_retry)
            }
            _ => policy.base_interval,
        }
    };
    now + delta
}

/// `min(base * 2^(n-1), cap)` — exponential, capped. Guards against overflow on
/// large `n`.
fn backoff(base: Duration, n: i32, cap: Duration) -> Duration {
    let shift = (n - 1).clamp(0, 20) as u32;
    let scaled = base.num_seconds().saturating_mul(1i64 << shift.min(62));
    let capped = scaled.min(cap.num_seconds());
    Duration::seconds(capped)
}

#[cfg(test)]
mod tests;
