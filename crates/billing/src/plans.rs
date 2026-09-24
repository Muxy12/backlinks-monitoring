//! The plan catalog (§39). Prices in USD cents. Limits per §39; `None` means
//! unlimited.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Plan {
    Starter,
    Pro,
    Agency,
    AgencyPlus,
}

/// The fixed specification of a plan. One source of truth (§39).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlanSpec {
    pub plan: Plan,
    /// Slug used in config/env for the Stripe price id lookup.
    pub slug: &'static str,
    pub display_name: &'static str,
    pub price_cents: u32,
    /// Monitored-backlink cap. `None` = unlimited.
    pub backlink_limit: Option<u32>,
    /// Project cap. `None` = unlimited.
    pub project_limit: Option<u32>,
    /// Team seats included. `None` = unlimited.
    pub seat_limit: Option<u32>,
}

impl Plan {
    /// This plan's fixed spec (§39).
    pub const fn spec(self) -> PlanSpec {
        match self {
            Plan::Starter => PlanSpec {
                plan: Plan::Starter,
                slug: "starter",
                display_name: "Starter",
                price_cents: 1900,
                backlink_limit: Some(1_000),
                project_limit: Some(3),
                seat_limit: Some(1),
            },
            Plan::Pro => PlanSpec {
                plan: Plan::Pro,
                slug: "pro",
                display_name: "Pro",
                price_cents: 4900,
                backlink_limit: Some(5_000),
                project_limit: Some(20),
                seat_limit: Some(3),
            },
            Plan::Agency => PlanSpec {
                plan: Plan::Agency,
                slug: "agency",
                display_name: "Agency",
                price_cents: 9900,
                backlink_limit: Some(20_000),
                project_limit: Some(100),
                seat_limit: Some(10),
            },
            Plan::AgencyPlus => PlanSpec {
                plan: Plan::AgencyPlus,
                slug: "agency_plus",
                display_name: "Agency+",
                price_cents: 19900,
                backlink_limit: Some(75_000),
                project_limit: None,
                seat_limit: None,
            },
        }
    }

    /// All plans, cheapest first — for pricing pages and upgrade ordering.
    pub const ALL: [Plan; 4] = [Plan::Starter, Plan::Pro, Plan::Agency, Plan::AgencyPlus];

    /// Resolve a plan from its slug (e.g. from the database enum or config).
    pub fn from_slug(slug: &str) -> Option<Plan> {
        Plan::ALL.into_iter().find(|p| p.spec().slug == slug)
    }

    /// Rank for comparing plan tiers (upgrade vs downgrade).
    pub const fn tier(self) -> u8 {
        match self {
            Plan::Starter => 0,
            Plan::Pro => 1,
            Plan::Agency => 2,
            Plan::AgencyPlus => 3,
        }
    }

    pub fn is_upgrade_to(self, other: Plan) -> bool {
        other.tier() > self.tier()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_matches_spec_section_39() {
        assert_eq!(Plan::Starter.spec().price_cents, 1900);
        assert_eq!(Plan::Starter.spec().backlink_limit, Some(1_000));
        assert_eq!(Plan::Pro.spec().price_cents, 4900);
        assert_eq!(Plan::Pro.spec().backlink_limit, Some(5_000));
        assert_eq!(Plan::Agency.spec().price_cents, 9900);
        assert_eq!(Plan::Agency.spec().backlink_limit, Some(20_000));
        assert_eq!(Plan::AgencyPlus.spec().price_cents, 19900);
        assert_eq!(Plan::AgencyPlus.spec().backlink_limit, Some(75_000));
        assert_eq!(
            Plan::AgencyPlus.spec().project_limit,
            None,
            "Agency+ has unlimited projects"
        );
    }

    #[test]
    fn slugs_round_trip() {
        for p in Plan::ALL {
            assert_eq!(Plan::from_slug(p.spec().slug), Some(p));
        }
        assert_eq!(Plan::from_slug("enterprise"), None);
    }

    #[test]
    fn tiers_order_and_upgrade_direction() {
        assert!(Plan::Starter.is_upgrade_to(Plan::Pro));
        assert!(Plan::Pro.is_upgrade_to(Plan::AgencyPlus));
        assert!(!Plan::Agency.is_upgrade_to(Plan::Pro));
        assert!(!Plan::Pro.is_upgrade_to(Plan::Pro));
    }

    #[test]
    fn price_increases_with_tier() {
        let mut last = 0;
        for p in Plan::ALL {
            assert!(p.spec().price_cents > last, "prices must increase by tier");
            last = p.spec().price_cents;
        }
    }
}
