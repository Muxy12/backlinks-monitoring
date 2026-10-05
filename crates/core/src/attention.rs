//! Attention prioritization (Overview "Needs Your Attention", §10/§56/§57).
//!
//! The ONE place that decides how urgent an actionable dashboard item is. Pure,
//! deterministic, no I/O — the API layer gathers the raw domain signals from the
//! database, asks this module for each item's `Priority`, then sorts the items by
//! (priority, economic value at risk, recency). Keeping the rules here (not in UI
//! or SQL) lets the same logic later feed notifications / digests / mobile alerts.
//!
//! This decides PRIORITY + ORDERING only; presentation (titles, routes, currency)
//! stays in the route, so this module needs no uuid/serde/db dependencies.

/// How urgent an attention item is. `rank()` gives the sort weight (higher first).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Priority {
    Critical,
    High,
    Medium,
    Low,
}

impl Priority {
    pub fn as_str(self) -> &'static str {
        match self {
            Priority::Critical => "critical",
            Priority::High => "high",
            Priority::Medium => "medium",
            Priority::Low => "low",
        }
    }
    /// Sort weight — higher sorts first.
    pub fn rank(self) -> u8 {
        match self {
            Priority::Critical => 3,
            Priority::High => 2,
            Priority::Medium => 1,
            Priority::Low => 0,
        }
    }
}

/// The kinds of actionable item the Overview surfaces.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttentionKind {
    /// A monitored backlink was lost.
    BacklinkLost,
    /// A monitored backlink changed in a bad way (nofollow, target/anchor change,
    /// redirect, source/target error).
    BacklinkChanged,
    /// A warranty/guarantee is lapsing soon.
    WarrantyExpiring,
    /// Campaign prospects are waiting for a decision (contacted / negotiating).
    ProspectsAwaiting,
    /// A campaign's budget is nearly exhausted.
    CampaignBudgetLow,
    /// A high-scoring opportunity is ready to review.
    HighValueOpportunity,
}

impl AttentionKind {
    pub fn as_str(self) -> &'static str {
        match self {
            AttentionKind::BacklinkLost => "backlink_lost",
            AttentionKind::BacklinkChanged => "backlink_changed",
            AttentionKind::WarrantyExpiring => "warranty_expiring",
            AttentionKind::ProspectsAwaiting => "prospects_awaiting",
            AttentionKind::CampaignBudgetLow => "campaign_budget_low",
            AttentionKind::HighValueOpportunity => "high_value_opportunity",
        }
    }
}

/// The priority of one item. `has_value` = there is money at stake (a known
/// acquisition cost); financial exposure escalates the item. First match wins.
pub fn priority(kind: AttentionKind, has_value: bool) -> Priority {
    match kind {
        // Losing a paid link is the most urgent thing on the dashboard.
        AttentionKind::BacklinkLost if has_value => Priority::Critical,
        AttentionKind::BacklinkLost => Priority::High,
        AttentionKind::BacklinkChanged if has_value => Priority::High,
        AttentionKind::BacklinkChanged => Priority::Medium,
        AttentionKind::WarrantyExpiring if has_value => Priority::High,
        AttentionKind::WarrantyExpiring => Priority::Medium,
        AttentionKind::ProspectsAwaiting => Priority::Medium,
        AttentionKind::CampaignBudgetLow => Priority::Medium,
        AttentionKind::HighValueOpportunity => Priority::Low,
    }
}

/// The descending sort key for an item: priority, then money at risk (in minor
/// units to keep it integer-ordered), then recency. Callers
/// `sort_by(|a, b| b.key().cmp(&a.key()))` (or sort keys descending).
pub fn sort_key(p: Priority, value_at_risk_units: i64, recency_secs: i64) -> (u8, i64, i64) {
    // Negate recency so that, at equal priority/value, the MORE recent (smaller
    // age) item sorts first under a descending sort.
    (p.rank(), value_at_risk_units, -recency_secs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lost_paid_link_is_critical() {
        assert_eq!(
            priority(AttentionKind::BacklinkLost, true),
            Priority::Critical
        );
    }

    #[test]
    fn lost_free_link_is_high_not_critical() {
        assert_eq!(priority(AttentionKind::BacklinkLost, false), Priority::High);
    }

    #[test]
    fn value_escalates_changed_and_warranty() {
        assert_eq!(
            priority(AttentionKind::BacklinkChanged, true),
            Priority::High
        );
        assert_eq!(
            priority(AttentionKind::BacklinkChanged, false),
            Priority::Medium
        );
        assert_eq!(
            priority(AttentionKind::WarrantyExpiring, true),
            Priority::High
        );
        assert_eq!(
            priority(AttentionKind::WarrantyExpiring, false),
            Priority::Medium
        );
    }

    #[test]
    fn opportunity_is_lowest() {
        assert_eq!(
            priority(AttentionKind::HighValueOpportunity, true),
            Priority::Low
        );
    }

    #[test]
    fn a_lost_paid_link_outranks_a_reply_waiting() {
        // The canonical example from the spec: "Paid backlink lost / €300 at risk"
        // must sort before "New prospect available".
        let lost = sort_key(priority(AttentionKind::BacklinkLost, true), 30000, 60);
        let reply = sort_key(priority(AttentionKind::ProspectsAwaiting, false), 0, 10);
        assert!(lost > reply);
    }

    #[test]
    fn higher_value_sorts_first_at_equal_priority() {
        let big = sort_key(Priority::Critical, 50000, 100);
        let small = sort_key(Priority::Critical, 10000, 100);
        assert!(big > small);
    }

    #[test]
    fn more_recent_sorts_first_at_equal_priority_and_value() {
        let recent = sort_key(Priority::High, 1000, 30);
        let older = sort_key(Priority::High, 1000, 9000);
        assert!(recent > older);
    }
}
