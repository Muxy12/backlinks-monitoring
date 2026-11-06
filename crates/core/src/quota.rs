//! Plan quota decisions for acquisition work (the margin guard). Pure: a cap
//! and current usage in, an allow/block decision with the remaining balance and
//! a reason out — no I/O, no clock, no DB.
//!
//! The rule the whole economic model leans on (docs/product/21 §6): acquisition
//! work (discovery, GEO captures) is capped per plan and checked **at enqueue**.
//! Over quota blocks the *new* job and is surfaced as an API `429` — it NEVER
//! deletes data and NEVER pauses monitoring. The monitoring/retention layer is
//! sacred and is never routed through this check.
//!
//! This is the one place the allow/block maths lives, so the enqueue path, the
//! REST `429`, and the `/usage` display all agree. The caps themselves are the
//! billing crate's `PlanSpec` (one source of truth); this module only compares.
//! A `None` cap means unlimited.

/// A metered acquisition resource. Monitoring is deliberately absent — it is
/// never quota-gated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuotaKind {
    /// A discovery run (theme → SERP → enrich) consuming provider units.
    Discoveries,
    /// A GEO capture (AI Overview lookup) consuming provider units.
    GeoQueries,
}

impl QuotaKind {
    /// Stable slug for API errors, headers and usage counters.
    pub const fn slug(self) -> &'static str {
        match self {
            QuotaKind::Discoveries => "discoveries",
            QuotaKind::GeoQueries => "geo_queries",
        }
    }
}

/// The verdict for one enqueue request against one quota.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuotaDecision {
    /// Whether this request fits within the remaining balance.
    pub allowed: bool,
    /// The plan cap for the period; `None` = unlimited.
    pub limit: Option<u32>,
    /// Usage already consumed this period.
    pub used: u32,
    /// Balance left after `used`; `None` = unlimited.
    pub remaining: Option<u32>,
    /// Human-readable rationale (stable strings).
    pub reason: &'static str,
}

/// Decide whether enqueuing work that would consume `requested` units of a
/// quota is allowed, given the plan `limit` and the `used` amount this period.
///
/// Deterministic. Over-limit returns `allowed: false` — the caller surfaces a
/// `429` and leaves existing data and monitoring untouched (§6). Saturating
/// arithmetic, so a `used` that already exceeds the cap simply reports zero
/// remaining rather than underflowing.
pub fn check(limit: Option<u32>, used: u32, requested: u32) -> QuotaDecision {
    match limit {
        None => QuotaDecision {
            allowed: true,
            limit: None,
            used,
            remaining: None,
            reason: "unlimited on this plan",
        },
        Some(cap) => {
            let remaining = cap.saturating_sub(used);
            let allowed = requested <= remaining;
            QuotaDecision {
                allowed,
                limit: Some(cap),
                used,
                remaining: Some(remaining),
                reason: if allowed {
                    "within quota"
                } else {
                    "quota exhausted for this period"
                },
            }
        }
    }
}

/// Convenience: would a single unit of work be allowed right now?
pub fn would_allow(limit: Option<u32>, used: u32) -> bool {
    check(limit, used, 1).allowed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unlimited_always_allows() {
        let d = check(None, 1_000_000, 50);
        assert!(d.allowed);
        assert_eq!(d.remaining, None);
        assert_eq!(d.limit, None);
        assert!(would_allow(None, u32::MAX));
    }

    #[test]
    fn within_quota_allows_and_reports_remaining() {
        let d = check(Some(100), 30, 10);
        assert!(d.allowed);
        assert_eq!(d.remaining, Some(70));
        assert_eq!(d.reason, "within quota");
    }

    #[test]
    fn exact_boundary_is_allowed() {
        // Exactly enough remaining for the request.
        let d = check(Some(100), 90, 10);
        assert!(d.allowed);
        assert_eq!(d.remaining, Some(10));
    }

    #[test]
    fn one_over_the_boundary_blocks() {
        let d = check(Some(100), 91, 10);
        assert!(!d.allowed);
        assert_eq!(d.remaining, Some(9));
        assert_eq!(d.reason, "quota exhausted for this period");
    }

    #[test]
    fn used_at_cap_blocks_further_work() {
        assert!(!would_allow(Some(100), 100));
        let d = check(Some(100), 100, 1);
        assert!(!d.allowed);
        assert_eq!(d.remaining, Some(0));
    }

    #[test]
    fn used_over_cap_saturates_to_zero_remaining() {
        // Never underflows, even if usage somehow exceeded the cap.
        let d = check(Some(100), 150, 1);
        assert!(!d.allowed);
        assert_eq!(d.remaining, Some(0));
    }

    #[test]
    fn zero_request_always_fits() {
        // A no-op enqueue is never blocked, even at the cap.
        assert!(check(Some(100), 100, 0).allowed);
    }

    #[test]
    fn kind_slugs_are_stable() {
        assert_eq!(QuotaKind::Discoveries.slug(), "discoveries");
        assert_eq!(QuotaKind::GeoQueries.slug(), "geo_queries");
    }

    #[test]
    fn deterministic() {
        assert_eq!(check(Some(100), 30, 10), check(Some(100), 30, 10));
    }
}
