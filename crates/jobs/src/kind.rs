//! Job kinds and their priorities (docs/07 §5). `check_backlink_now` is the
//! only user-facing latency path and jumps the queue; everything else is
//! background.

use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobKind {
    /// The deduped unit of work: one fetch, N backlink evaluations (§27).
    CheckSourcePage,
    /// User-triggered "Check now" (§36) — jumps the queue.
    CheckBacklinkNow,
    /// Deduped target-URL checks.
    CheckTargetUrl,
    /// The scheduler tick (§11).
    ScheduleDueChecks,
    /// Parse an upload and fan out to row chunks (§38).
    ProcessImport,
    /// A bounded slice of import rows.
    ImportRowsChunk,
    /// Deliver one notification (§37).
    SendNotification,
    /// Batch notifications for an org over a window (§37).
    DigestNotifications,
    /// Daily warranty-expiry sweep (§32).
    CheckWarranties,
    /// Recompute an org's usage counters (§39).
    RecomputeUsage,
    /// Nightly `backlink_checks` downsampling (docs/03 §3.5).
    RetentionDownsample,
}

impl JobKind {
    /// Queue priority — higher runs first (docs/07 §5). Only the user-facing
    /// check is elevated; the scheduler sits above ordinary background work so
    /// due checks get enqueued promptly.
    pub const fn priority(self) -> i32 {
        match self {
            JobKind::CheckBacklinkNow => 100,
            JobKind::ScheduleDueChecks => 50,
            JobKind::SendNotification => 30,
            JobKind::ProcessImport | JobKind::ImportRowsChunk => 20,
            JobKind::DigestNotifications | JobKind::CheckWarranties | JobKind::RecomputeUsage => 10,
            JobKind::CheckSourcePage | JobKind::CheckTargetUrl => 0,
            JobKind::RetentionDownsample => -10,
        }
    }

    /// The stable string stored in `jobs.job_type`.
    pub const fn as_str(self) -> &'static str {
        match self {
            JobKind::CheckSourcePage => "check_source_page",
            JobKind::CheckBacklinkNow => "check_backlink_now",
            JobKind::CheckTargetUrl => "check_target_url",
            JobKind::ScheduleDueChecks => "schedule_due_checks",
            JobKind::ProcessImport => "process_import",
            JobKind::ImportRowsChunk => "import_rows_chunk",
            JobKind::SendNotification => "send_notification",
            JobKind::DigestNotifications => "digest_notifications",
            JobKind::CheckWarranties => "check_warranties",
            JobKind::RecomputeUsage => "recompute_usage",
            JobKind::RetentionDownsample => "retention_downsample",
        }
    }
}

impl FromStr for JobKind {
    type Err = ();

    fn from_str(s: &str) -> Result<JobKind, ()> {
        use JobKind::*;
        Ok(match s {
            "check_source_page" => CheckSourcePage,
            "check_backlink_now" => CheckBacklinkNow,
            "check_target_url" => CheckTargetUrl,
            "schedule_due_checks" => ScheduleDueChecks,
            "process_import" => ProcessImport,
            "import_rows_chunk" => ImportRowsChunk,
            "send_notification" => SendNotification,
            "digest_notifications" => DigestNotifications,
            "check_warranties" => CheckWarranties,
            "recompute_usage" => RecomputeUsage,
            "retention_downsample" => RetentionDownsample,
            _ => return Err(()),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_now_is_the_top_priority() {
        assert_eq!(JobKind::CheckBacklinkNow.priority(), 100);
        for k in [
            JobKind::CheckSourcePage,
            JobKind::ProcessImport,
            JobKind::ScheduleDueChecks,
        ] {
            assert!(k.priority() < JobKind::CheckBacklinkNow.priority());
        }
    }

    #[test]
    fn retention_is_lowest() {
        assert_eq!(JobKind::RetentionDownsample.priority(), -10);
    }

    #[test]
    fn string_round_trips_for_every_kind() {
        for k in [
            JobKind::CheckSourcePage,
            JobKind::CheckBacklinkNow,
            JobKind::CheckTargetUrl,
            JobKind::ScheduleDueChecks,
            JobKind::ProcessImport,
            JobKind::ImportRowsChunk,
            JobKind::SendNotification,
            JobKind::DigestNotifications,
            JobKind::CheckWarranties,
            JobKind::RecomputeUsage,
            JobKind::RetentionDownsample,
        ] {
            assert_eq!(k.as_str().parse::<JobKind>(), Ok(k), "round-trip for {k:?}");
        }
        assert_eq!("nonsense".parse::<JobKind>(), Err(()));
    }
}
