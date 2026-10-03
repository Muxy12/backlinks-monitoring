//! Entitlement rules: what a plan's current usage does and does not allow
//! (§39). The load-bearing rule: **exceeding a limit blocks new backlinks but
//! never stops monitoring existing ones** (§39, docs/13 Phase 6). A downgrade
//! that puts usage over the new cap is allowed to persist — we stop new adds,
//! we do not delete the customer's data.

use crate::plans::Plan;

/// The result of asking "can this org add `n` more backlinks?".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddOutcome {
    /// Allowed; `remaining` is the headroom left *after* this add (None = ∞).
    Allowed { remaining: Option<u32> },
    /// Rejected; the add would exceed the plan cap. `limit`/`current` explain it.
    OverLimit { limit: u32, current: u32 },
}

impl AddOutcome {
    pub fn is_allowed(self) -> bool {
        matches!(self, AddOutcome::Allowed { .. })
    }
}

/// A view over an org's plan and current usage. Cheap to construct per request.
#[derive(Debug, Clone, Copy)]
pub struct Entitlements {
    pub plan: Plan,
}

impl Entitlements {
    pub fn new(plan: Plan) -> Self {
        Entitlements { plan }
    }

    /// Can the org create `count` more backlinks given `current` monitored?
    /// Unlimited plans always allow. Over-limit blocks the *new* adds only.
    pub fn can_add_backlinks(&self, current: u32, count: u32) -> AddOutcome {
        match self.plan.spec().backlink_limit {
            None => AddOutcome::Allowed { remaining: None },
            Some(limit) => {
                // saturating so a huge `count` cannot wrap.
                if current.saturating_add(count) <= limit {
                    AddOutcome::Allowed {
                        remaining: Some(limit - current.saturating_add(count)),
                    }
                } else {
                    AddOutcome::OverLimit { limit, current }
                }
            }
        }
    }

    /// Can the org create one more project?
    pub fn can_add_project(&self, current: u32) -> AddOutcome {
        match self.plan.spec().project_limit {
            None => AddOutcome::Allowed { remaining: None },
            Some(limit) if current < limit => AddOutcome::Allowed {
                remaining: Some(limit - current - 1),
            },
            Some(limit) => AddOutcome::OverLimit { limit, current },
        }
    }

    /// Can the org invite one more member?
    pub fn can_add_seat(&self, current: u32) -> AddOutcome {
        match self.plan.spec().seat_limit {
            None => AddOutcome::Allowed { remaining: None },
            Some(limit) if current < limit => AddOutcome::Allowed {
                remaining: Some(limit - current - 1),
            },
            Some(limit) => AddOutcome::OverLimit { limit, current },
        }
    }

    /// True when current usage already exceeds the plan cap — e.g. right after a
    /// downgrade. Monitoring of existing links continues; only new adds stop.
    pub fn is_over_backlink_limit(&self, current: u32) -> bool {
        matches!(self.plan.spec().backlink_limit, Some(limit) if current > limit)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_within_limit_reports_remaining() {
        let e = Entitlements::new(Plan::Starter); // 500 backlinks
        assert_eq!(
            e.can_add_backlinks(400, 50),
            AddOutcome::Allowed {
                remaining: Some(50)
            }
        );
    }

    #[test]
    fn add_exactly_to_limit_is_allowed() {
        let e = Entitlements::new(Plan::Starter); // 500
        assert_eq!(
            e.can_add_backlinks(499, 1),
            AddOutcome::Allowed { remaining: Some(0) }
        );
    }

    #[test]
    fn add_over_limit_is_blocked_with_context() {
        let e = Entitlements::new(Plan::Starter); // 500
        assert_eq!(
            e.can_add_backlinks(500, 1),
            AddOutcome::OverLimit {
                limit: 500,
                current: 500
            }
        );
    }

    #[test]
    fn bulk_add_that_would_exceed_is_blocked_atomically() {
        let e = Entitlements::new(Plan::Pro); // 2500
        assert!(
            !e.can_add_backlinks(2490, 20).is_allowed(),
            "a 20-add crossing the cap is rejected"
        );
        assert!(e.can_add_backlinks(2490, 10).is_allowed());
    }

    #[test]
    fn downgrade_over_limit_keeps_monitoring_but_blocks_new() {
        // Org had 2000 backlinks on Pro, downgrades to Starter (500).
        let starter = Entitlements::new(Plan::Starter);
        assert!(
            starter.is_over_backlink_limit(2000),
            "existing usage is over the new cap"
        );
        // Existing links keep being monitored (nothing here deletes them);
        // but no new backlink may be added.
        assert!(
            !starter.can_add_backlinks(2000, 1).is_allowed(),
            "new adds blocked while over cap"
        );
    }

    #[test]
    fn saturating_add_guards_against_overflow() {
        let e = Entitlements::new(Plan::Starter);
        assert!(!e.can_add_backlinks(u32::MAX, u32::MAX).is_allowed());
    }

    #[test]
    fn project_and_seat_limits() {
        let starter = Entitlements::new(Plan::Starter); // 3 projects, 1 seat
        assert!(starter.can_add_project(2).is_allowed());
        assert!(!starter.can_add_project(3).is_allowed());
        assert!(!starter.can_add_seat(1).is_allowed());

        let business = Entitlements::new(Plan::Business); // 25 projects, 5 seats
        assert!(business.can_add_project(24).is_allowed());
        assert!(!business.can_add_project(25).is_allowed());
        assert!(business.can_add_seat(4).is_allowed());
        assert!(!business.can_add_seat(5).is_allowed());
    }
}
