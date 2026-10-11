//! Pure decision logic for link reclamation (BETA).
//!
//! When a once-valuable backlink loses its value — it is removed, turns
//! nofollow, or its source page errors out — the owner can often win it back:
//! that site linked them before. This module is the deterministic, I/O-free core
//! that decides *whether* a lost link is worth reclaiming and *how* to frame it.
//! The DB engine pass (`db::repositories::reclamation`) and the outreach draft
//! consume these decisions; keeping them pure makes the rules unit-testable and
//! identical on every run.
//!
//! Two product rules are enforced here, not just in SQL:
//!   * Only the owner's **own** links are reclaimed — never a competitor's lost
//!     link (we don't help a competitor win their link back).
//!   * A problem *we* caused — our target page moved or errors (`target_error`,
//!     `redirected`) — is a fix-it-yourself suggestion, not an outreach target,
//!     so those statuses are deliberately not reclaimable.

/// Why a once-live link is worth winning back — drives the pitch angle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReclaimAngle {
    /// The link was removed from the source page (status `lost`).
    Removed,
    /// The link is still there but became nofollow (status `nofollow`).
    Nofollowed,
    /// The source page itself errors (status `source_error`).
    SourcePageError,
}

impl ReclaimAngle {
    /// Map a backlink status to a reclamation angle, or `None` when the status is
    /// not reclaimable. Not reclaimable: `live`, `pending`, `unknown`, `changed`,
    /// `blocked`, `suspected_lost` (unconfirmed — never act on a transient), and
    /// — deliberately — `target_error`/`redirected`, which mean *our* target page
    /// moved or errored (our fault → fix it, don't email the publisher).
    pub fn from_status(status: &str) -> Option<Self> {
        match status {
            "lost" => Some(Self::Removed),
            "nofollow" => Some(Self::Nofollowed),
            "source_error" => Some(Self::SourcePageError),
            _ => None,
        }
    }

    /// Stable machine tag (stored/logged, never user-facing prose).
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Removed => "removed",
            Self::Nofollowed => "nofollowed",
            Self::SourcePageError => "source_page_error",
        }
    }

    /// Deterministic priority (0–100) + a short, explainable reason. A removed
    /// link from a prior linker is the hottest prospect; a nofollow is still warm.
    pub fn score(self) -> (i32, &'static str) {
        match self {
            Self::Removed => (95, "Lost link — this site linked you before"),
            Self::Nofollowed => (85, "Link turned nofollow — ask to restore dofollow"),
            Self::SourcePageError => (80, "Linking page is erroring — ask where it moved"),
        }
    }
}

/// A link is eligible for reclamation only when it is the owner's **own** link
/// and it had real value: it is actively monitored, it is `protected`, it cost
/// money, or it belongs to a campaign. Organic, inventory-only links never
/// trigger outreach automatically (same spirit as Post-Link Impact eligibility).
pub fn is_eligible(
    subject: &str,
    monitoring_enabled: bool,
    monitoring_level: &str,
    has_cost: bool,
    has_campaign: bool,
) -> bool {
    subject == "own"
        && (monitoring_enabled || monitoring_level == "protected" || has_cost || has_campaign)
}

/// The "receipts" stored on the prospect so the operator — and the outreach
/// draft — have the full context without re-querying: what happened, when, what
/// was linked, from where, and with which anchor.
pub fn receipts(
    angle: ReclaimAngle,
    source_url: &str,
    anchor: Option<&str>,
    target_url: &str,
    lost_on: Option<&str>,
) -> String {
    let what = match angle {
        ReclaimAngle::Removed => "Link removed",
        ReclaimAngle::Nofollowed => "Link turned nofollow",
        ReclaimAngle::SourcePageError => "Linking page erroring",
    };
    let when_part = match lost_on.map(str::trim).filter(|s| !s.is_empty()) {
        Some(d) => format!(" (detected {d})"),
        None => String::new(),
    };
    let anchor_part = match anchor.map(str::trim).filter(|s| !s.is_empty()) {
        Some(a) => format!(" with anchor \"{a}\""),
        None => String::new(),
    };
    format!("{what}{when_part}: {source_url} linked {target_url}{anchor_part}.")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_only_reclaimable_statuses() {
        assert_eq!(
            ReclaimAngle::from_status("lost"),
            Some(ReclaimAngle::Removed)
        );
        assert_eq!(
            ReclaimAngle::from_status("nofollow"),
            Some(ReclaimAngle::Nofollowed)
        );
        assert_eq!(
            ReclaimAngle::from_status("source_error"),
            Some(ReclaimAngle::SourcePageError)
        );
        // Not reclaimable — our fault, transient, or no loss.
        for s in [
            "live",
            "pending",
            "unknown",
            "changed",
            "blocked",
            "suspected_lost",
            "target_error",
            "redirected",
        ] {
            assert_eq!(
                ReclaimAngle::from_status(s),
                None,
                "status {s} must not reclaim"
            );
        }
    }

    #[test]
    fn eligibility_requires_own_and_value() {
        // Own + any value signal → eligible.
        assert!(is_eligible("own", true, "tracked", false, false));
        assert!(is_eligible("own", false, "protected", false, false));
        assert!(is_eligible("own", false, "inventory_only", true, false));
        assert!(is_eligible("own", false, "inventory_only", false, true));
        // Own but no value (organic inventory) → not eligible.
        assert!(!is_eligible("own", false, "inventory_only", false, false));
        // Competitor links are never reclaimed, whatever the value.
        assert!(!is_eligible("competitor", true, "protected", true, true));
    }

    #[test]
    fn score_is_ordered_removed_first() {
        assert!(ReclaimAngle::Removed.score().0 > ReclaimAngle::Nofollowed.score().0);
        assert!(ReclaimAngle::Nofollowed.score().0 > ReclaimAngle::SourcePageError.score().0);
    }

    #[test]
    fn receipts_include_anchor_and_date_when_present() {
        let r = receipts(
            ReclaimAngle::Removed,
            "https://blog.example/post",
            Some("best tools"),
            "https://me.example/x",
            Some("2026-10-01"),
        );
        assert!(r.contains("Link removed"));
        assert!(r.contains("detected 2026-10-01"));
        assert!(r.contains("https://blog.example/post"));
        assert!(r.contains("https://me.example/x"));
        assert!(r.contains("\"best tools\""));
    }

    #[test]
    fn receipts_omit_missing_optionals_cleanly() {
        let r = receipts(
            ReclaimAngle::Nofollowed,
            "https://blog.example/post",
            None,
            "https://me.example/x",
            None,
        );
        assert!(r.contains("Link turned nofollow"));
        assert!(!r.contains("detected"));
        assert!(!r.contains("anchor"));
    }
}
