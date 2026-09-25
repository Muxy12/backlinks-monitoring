//! Tests for the backlink state machine. The property test at the end is the
//! mechanical enforcement of §24 / §57.10.

use super::*;
use crate::observation::{BlockReason, ConnErrorKind};
use crate::status::RelSet;

fn now() -> DateTime<Utc> {
    DateTime::parse_from_rfc3339("2026-09-24T12:00:00Z")
        .unwrap()
        .with_timezone(&Utc)
}

fn policy() -> TransitionPolicy {
    TransitionPolicy::default()
}

fn live_state() -> BacklinkState {
    BacklinkState {
        status: BacklinkStatus::Live,
        expected_anchor_text: Some("seo tools".into()),
        expected_rel_type: RelType::Dofollow,
        detected: DetectedFields {
            anchor_text: Some("seo tools".into()),
            target_url: Some("https://example.com/x".into()),
            rel_attributes: vec![],
            is_dofollow: Some(true),
            canonical: None,
            noindex: false,
            is_image: false,
            image_alt: None,
        },
        consecutive_failure_count: 0,
        last_seen_at: Some(now() - Duration::days(7)),
    }
}

fn present(anchor: &str, dofollow: bool) -> Observation {
    let mut rel = RelSet::new();
    if !dofollow {
        rel.insert(RelType::Nofollow);
    }
    Observation::Present(Detected {
        anchor_text: Some(anchor.into()),
        target_url: Some("https://example.com/x".into()),
        rel,
        is_dofollow: dofollow,
        canonical: None,
        noindex: false,
        is_image: false,
        image_alt: None,
    })
}

fn has_event(t: &Transition, e: EventType) -> bool {
    t.events.iter().any(|ev| ev.event_type == e)
}

// ── Positive path ────────────────────────────────────────────────────────────

#[test]
fn pending_to_live_on_first_detection_emits_found() {
    let s = BacklinkState::pending(Some("seo tools".into()), RelType::Dofollow);
    let t = transition(&s, &present("seo tools", true), now(), &policy());
    assert_eq!(t.status, BacklinkStatus::Live);
    assert!(has_event(&t, EventType::BacklinkFound));
    assert_eq!(t.consecutive_failure_count, 0);
    assert_eq!(t.last_seen_at, Some(now()));
}

#[test]
fn stable_live_reschedules_at_base_interval() {
    let t = transition(&live_state(), &present("seo tools", true), now(), &policy());
    assert_eq!(t.status, BacklinkStatus::Live);
    assert_eq!(t.next_check_at, now() + Duration::days(7));
    assert!(t.events.is_empty(), "a no-change check emits no events");
}

// ── The core guarantee (§24): one failure never loses a link ─────────────────

#[test]
fn single_absent_does_not_lose_a_live_link() {
    let t = transition(&live_state(), &Observation::Absent, now(), &policy());
    assert_eq!(
        t.status,
        BacklinkStatus::Live,
        "status held after ONE absence"
    );
    assert_eq!(t.consecutive_failure_count, 1);
    assert!(!has_event(&t, EventType::BacklinkLost));
    // And it is rescheduled aggressively, not at the 7-day cadence.
    assert!(t.next_check_at < now() + Duration::days(1));
}

#[test]
fn two_consecutive_absent_confirms_lost() {
    let mut s = live_state();
    let t1 = transition(&s, &Observation::Absent, now(), &policy());
    assert_eq!(t1.status, BacklinkStatus::Live);

    s.status = t1.status;
    s.consecutive_failure_count = t1.consecutive_failure_count;
    let t2 = transition(&s, &Observation::Absent, now(), &policy());
    assert_eq!(t2.status, BacklinkStatus::Lost);
    assert!(has_event(&t2, EventType::BacklinkLost));
    assert_eq!(
        t2.next_check_at,
        now() + Duration::days(3),
        "keep watching lost links"
    );
}

#[test]
fn a_present_between_absences_resets_the_counter() {
    let mut s = live_state();
    let t1 = transition(&s, &Observation::Absent, now(), &policy());
    assert_eq!(t1.consecutive_failure_count, 1);

    s.consecutive_failure_count = t1.consecutive_failure_count;
    let t2 = transition(&s, &present("seo tools", true), now(), &policy());
    assert_eq!(
        t2.consecutive_failure_count, 0,
        "recovery resets the counter"
    );
    assert_eq!(t2.status, BacklinkStatus::Live);
}

// ── Errors are confirmed, and are distinct from "lost" ───────────────────────

#[test]
fn source_500_takes_three_confirmations() {
    let mut s = live_state();
    for i in 1..=2 {
        let t = transition(
            &s,
            &Observation::SourceHttpError { status: 500 },
            now(),
            &policy(),
        );
        assert_eq!(
            t.status,
            BacklinkStatus::Live,
            "still live after {i} error(s)"
        );
        s.consecutive_failure_count = t.consecutive_failure_count;
        s.status = t.status;
    }
    let t = transition(
        &s,
        &Observation::SourceHttpError { status: 500 },
        now(),
        &policy(),
    );
    assert_eq!(t.status, BacklinkStatus::SourceError);
    assert!(has_event(&t, EventType::SourceError));
}

#[test]
fn unreachable_becomes_unknown_not_lost() {
    let mut s = live_state();
    let mut last = BacklinkStatus::Live;
    for _ in 0..3 {
        let t = transition(
            &s,
            &Observation::SourceUnreachable {
                kind: ConnErrorKind::Timeout,
            },
            now(),
            &policy(),
        );
        s.consecutive_failure_count = t.consecutive_failure_count;
        s.status = t.status;
        last = t.status;
    }
    assert_eq!(
        last,
        BacklinkStatus::Unknown,
        "network failure is 'unknown', never 'lost'"
    );
}

#[test]
fn blocked_reschedules_far_out() {
    let mut s = live_state();
    let t1 = transition(
        &s,
        &Observation::SourceBlocked {
            reason: BlockReason::RobotsTxt,
        },
        now(),
        &policy(),
    );
    s.consecutive_failure_count = t1.consecutive_failure_count;
    s.status = t1.status;
    let t2 = transition(
        &s,
        &Observation::SourceBlocked {
            reason: BlockReason::RobotsTxt,
        },
        now(),
        &policy(),
    );
    assert_eq!(t2.status, BacklinkStatus::Blocked);
    assert_eq!(t2.next_check_at, now() + Duration::days(30));
}

// ── Change detection ─────────────────────────────────────────────────────────

#[test]
fn anchor_change_sets_changed_and_emits_event() {
    let t = transition(
        &live_state(),
        &present("different anchor", true),
        now(),
        &policy(),
    );
    assert_eq!(t.status, BacklinkStatus::Changed);
    assert!(has_event(&t, EventType::AnchorChanged));
}

#[test]
fn dofollow_to_nofollow_sets_nofollow_and_emits_both_signals() {
    // Anchor unchanged, follow dropped: status nofollow, event for the drop.
    let t = transition(
        &live_state(),
        &present("seo tools", false),
        now(),
        &policy(),
    );
    assert_eq!(t.status, BacklinkStatus::Nofollow);
    assert!(has_event(&t, EventType::DofollowToNofollow));
}

#[test]
fn changed_outranks_nofollow_but_both_events_fire() {
    // A link that changed anchor AND went nofollow: status is `changed`
    // (higher precedence), but both events are recorded (docs/08 §2.1).
    let t = transition(
        &live_state(),
        &present("new anchor", false),
        now(),
        &policy(),
    );
    assert_eq!(t.status, BacklinkStatus::Changed);
    assert!(has_event(&t, EventType::AnchorChanged));
    assert!(has_event(&t, EventType::DofollowToNofollow));
}

#[test]
fn anchor_event_compares_to_previous_detection_not_expectation() {
    // Import a link whose expectation is wrong from day one: detected anchor
    // never matches expected, but since detection is stable, no AnchorChanged
    // event fires on the second check (docs/08 §6.1).
    let mut s = live_state();
    s.expected_anchor_text = Some("what we hoped for".into());
    // detected already "seo tools"; observe "seo tools" again.
    let t = transition(&s, &present("seo tools", true), now(), &policy());
    assert!(
        !has_event(&t, EventType::AnchorChanged),
        "stable detection => no change event"
    );
    // status is still `changed` because detection != expectation.
    assert_eq!(t.status, BacklinkStatus::Changed);
}

#[test]
fn recovery_from_lost_emits_restored() {
    let mut s = live_state();
    s.status = BacklinkStatus::Lost;
    let t = transition(&s, &present("seo tools", true), now(), &policy());
    assert_eq!(t.status, BacklinkStatus::Live);
    assert!(has_event(&t, EventType::BacklinkRestored));
}

#[test]
fn redirect_is_immediate_and_emits_event() {
    let t = transition(
        &live_state(),
        &Observation::SourceRedirected {
            to: "https://pub.com/moved".into(),
            permanent: true,
        },
        now(),
        &policy(),
    );
    assert_eq!(t.status, BacklinkStatus::Redirected);
    assert!(has_event(&t, EventType::SourceRedirected));
}

#[test]
fn noindex_and_canonical_changes_emit_events() {
    let mut s = live_state();
    s.detected.canonical = Some("https://example.com/x".into());
    let obs = Observation::Present(Detected {
        anchor_text: Some("seo tools".into()),
        target_url: Some("https://example.com/x".into()),
        rel: RelSet::new(),
        is_dofollow: true,
        canonical: Some("https://example.com/other".into()),
        noindex: true,
        is_image: false,
        image_alt: None,
    });
    let t = transition(&s, &obs, now(), &policy());
    assert!(has_event(&t, EventType::CanonicalChanged));
    assert!(has_event(&t, EventType::NoindexDetected));
}

// ── Determinism ──────────────────────────────────────────────────────────────

#[test]
fn transition_is_pure() {
    let s = live_state();
    let a = transition(&s, &present("seo tools", true), now(), &policy());
    let b = transition(&s, &present("seo tools", true), now(), &policy());
    assert_eq!(a, b, "same inputs must give the same output");
}

// ── THE property test: §24 / §57.10 mechanically enforced ────────────────────

#[test]
fn only_absent_can_ever_produce_lost() {
    // Every non-Absent observation, repeated many times from every starting
    // state, must never yield `lost`. This is the product's core promise
    // expressed as a machine-checked invariant (docs/08 §5, property 2).
    let non_absent: Vec<Observation> = vec![
        present("seo tools", true),
        present("changed", false),
        Observation::SourceGone { status: 404 },
        Observation::SourceHttpError { status: 500 },
        Observation::SourceHttpError { status: 503 },
        Observation::SourceUnreachable {
            kind: ConnErrorKind::Timeout,
        },
        Observation::SourceUnreachable {
            kind: ConnErrorKind::Dns,
        },
        Observation::SourceUnreachable {
            kind: ConnErrorKind::Tls,
        },
        Observation::SourceBlocked {
            reason: BlockReason::RobotsTxt,
        },
        Observation::SourceBlocked {
            reason: BlockReason::SsrfGuard,
        },
        Observation::SourceRedirected {
            to: "https://pub.com/y".into(),
            permanent: false,
        },
    ];

    let start_states = [
        BacklinkStatus::Pending,
        BacklinkStatus::Live,
        BacklinkStatus::Changed,
        BacklinkStatus::Nofollow,
        BacklinkStatus::SourceError,
        BacklinkStatus::TargetError,
        BacklinkStatus::Blocked,
        BacklinkStatus::Unknown,
        BacklinkStatus::Redirected,
    ];

    for &start in &start_states {
        for obs in &non_absent {
            let mut s = live_state();
            s.status = start;
            s.consecutive_failure_count = 0;
            // Repeat the SAME non-absent observation far past every threshold.
            for _ in 0..25 {
                let t = transition(&s, obs, now(), &policy());
                assert_ne!(
                    t.status,
                    BacklinkStatus::Lost,
                    "observation {obs:?} from {start:?} produced LOST — violates §24",
                );
                s.status = t.status;
                s.consecutive_failure_count = t.consecutive_failure_count;
                s.detected = t.detected;
            }
        }
    }
}

#[test]
fn absent_is_the_documented_path_to_lost() {
    // The positive counterpart: repeated Absent DOES reach lost, at the
    // configured threshold and not before.
    let p = policy();
    let mut s = live_state();
    let mut reached_at = None;
    for i in 1..=5 {
        let t = transition(&s, &Observation::Absent, now(), &p);
        if t.status == BacklinkStatus::Lost {
            reached_at = Some(i);
            break;
        }
        s.status = t.status;
        s.consecutive_failure_count = t.consecutive_failure_count;
    }
    assert_eq!(
        reached_at,
        Some(p.absent_to_lost),
        "lost at exactly the threshold"
    );
}
