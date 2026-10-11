//! Pure scheduling for outreach sequences (BETA).
//!
//! Decides, for one enrolled prospect, whether the next follow-up is due now,
//! still pending, or the sequence is complete. I/O-free and deterministic so the
//! cadence is unit-tested and identical every run; the DB dispatch pass
//! (`db::repositories::outreach`) and the tick consume these decisions.
//!
//! The cadence is a fixed, gentle 3-touch sequence. "Stop-on-reply" is enforced
//! upstream: the dispatch only ever considers prospects still in the `contacted`
//! stage, so the moment a reply moves the prospect forward the sequence halts.

use chrono::{DateTime, Duration, Utc};

/// Day offsets from enrollment for each step: first touch, then +3 and +7 days.
/// Changing this changes the cadence for every in-flight sequence.
pub const STEP_OFFSETS_DAYS: &[i64] = &[0, 3, 7];

/// What the dispatch should do for one enrolled prospect right now.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DispatchAction {
    /// Send this 1-based step now.
    Send(u8),
    /// The next step is scheduled but not due yet — leave it.
    Wait,
    /// Every step has been sent; the sequence is complete.
    Done,
}

/// Decide the next action given how many steps have already been sent (`sent`,
/// a count in `0..=STEP_OFFSETS_DAYS.len()`) and when the prospect was enrolled
/// (its first-contact time). Deterministic.
pub fn decide(now: DateTime<Utc>, enrolled_at: DateTime<Utc>, sent: usize) -> DispatchAction {
    if sent >= STEP_OFFSETS_DAYS.len() {
        return DispatchAction::Done;
    }
    let due = enrolled_at + Duration::days(STEP_OFFSETS_DAYS[sent]);
    if now >= due {
        // sent == index of the next step; steps are reported 1-based.
        DispatchAction::Send((sent + 1) as u8)
    } else {
        DispatchAction::Wait
    }
}

/// Total number of steps in the cadence.
pub fn step_count() -> usize {
    STEP_OFFSETS_DAYS.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(s: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(s).unwrap().with_timezone(&Utc)
    }

    #[test]
    fn first_step_is_due_at_enrollment() {
        let enrolled = t("2026-10-01T00:00:00Z");
        assert_eq!(decide(enrolled, enrolled, 0), DispatchAction::Send(1));
    }

    #[test]
    fn second_step_waits_until_plus_three_days() {
        let enrolled = t("2026-10-01T00:00:00Z");
        // One step sent; +2 days is too early, +3 is due.
        assert_eq!(
            decide(t("2026-10-03T00:00:00Z"), enrolled, 1),
            DispatchAction::Wait
        );
        assert_eq!(
            decide(t("2026-10-04T00:00:00Z"), enrolled, 1),
            DispatchAction::Send(2)
        );
    }

    #[test]
    fn third_step_due_at_plus_seven_days() {
        let enrolled = t("2026-10-01T00:00:00Z");
        assert_eq!(
            decide(t("2026-10-07T12:00:00Z"), enrolled, 2),
            DispatchAction::Wait
        );
        assert_eq!(
            decide(t("2026-10-08T00:00:00Z"), enrolled, 2),
            DispatchAction::Send(3)
        );
    }

    #[test]
    fn sequence_is_done_after_all_steps() {
        let enrolled = t("2026-10-01T00:00:00Z");
        assert_eq!(
            decide(t("2027-01-01T00:00:00Z"), enrolled, 3),
            DispatchAction::Done
        );
        // Defensive: never panics or re-sends past the end.
        assert_eq!(
            decide(t("2027-01-01T00:00:00Z"), enrolled, 9),
            DispatchAction::Done
        );
    }

    #[test]
    fn step_count_matches_offsets() {
        assert_eq!(step_count(), 3);
    }
}
