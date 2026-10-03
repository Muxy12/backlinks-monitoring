//! The plan catalog (§39) — THE single source of truth for plan prices, limits,
//! monitoring interval and the "most popular" flag. Prices in USD cents.
//! `None` limit = unlimited. Every plan comes with a 10-day free trial, no card
//! required (see docs/20 — pricing & trial). There is no permanent free plan.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Plan {
    Starter,
    Pro,
    Business,
    Agency,
}

/// The fixed specification of a plan. One source of truth (§39).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlanSpec {
    pub plan: Plan,
    /// Slug used in the DB enum, config/env and the API.
    pub slug: &'static str,
    pub display_name: &'static str,
    /// Monthly price, USD cents.
    pub price_cents: u32,
    /// Annual price, USD cents (~2 months free vs monthly × 12).
    pub annual_price_cents: u32,
    /// Monitored-backlink cap. `None` = unlimited.
    pub backlink_limit: Option<u32>,
    /// Project cap. `None` = unlimited.
    pub project_limit: Option<u32>,
    /// Team seats included (owner counts as a seat). `None` = unlimited.
    pub seat_limit: Option<u32>,
    /// How often this plan's backlinks are monitored.
    pub monitoring_interval_hours: u32,
    /// Highlighted as "Most popular" on the pricing page.
    pub most_popular: bool,
}

impl Plan {
    /// This plan's fixed spec (§39).
    pub const fn spec(self) -> PlanSpec {
        match self {
            Plan::Starter => PlanSpec {
                plan: Plan::Starter,
                slug: "starter",
                display_name: "Starter",
                price_cents: 2900,
                annual_price_cents: 29000,
                backlink_limit: Some(500),
                project_limit: Some(3),
                seat_limit: Some(1),
                monitoring_interval_hours: 48,
                most_popular: false,
            },
            Plan::Pro => PlanSpec {
                plan: Plan::Pro,
                slug: "pro",
                display_name: "Pro",
                price_cents: 5900,
                annual_price_cents: 59000,
                backlink_limit: Some(2_500),
                project_limit: Some(10),
                seat_limit: Some(3),
                monitoring_interval_hours: 48,
                most_popular: true,
            },
            Plan::Business => PlanSpec {
                plan: Plan::Business,
                slug: "business",
                display_name: "Business",
                price_cents: 9900,
                annual_price_cents: 99000,
                backlink_limit: Some(5_000),
                project_limit: Some(25),
                seat_limit: Some(5),
                monitoring_interval_hours: 48,
                most_popular: false,
            },
            Plan::Agency => PlanSpec {
                plan: Plan::Agency,
                slug: "agency",
                display_name: "Agency",
                price_cents: 19900,
                annual_price_cents: 199000,
                backlink_limit: Some(15_000),
                project_limit: Some(75),
                seat_limit: Some(15),
                monitoring_interval_hours: 48,
                most_popular: false,
            },
        }
    }

    /// All plans, cheapest first — for pricing pages and upgrade ordering.
    pub const ALL: [Plan; 4] = [Plan::Starter, Plan::Pro, Plan::Business, Plan::Agency];

    /// Resolve a plan from its slug (e.g. from the database enum or config).
    pub fn from_slug(slug: &str) -> Option<Plan> {
        Plan::ALL.into_iter().find(|p| p.spec().slug == slug)
    }

    /// Rank for comparing plan tiers (upgrade vs downgrade).
    pub const fn tier(self) -> u8 {
        match self {
            Plan::Starter => 0,
            Plan::Pro => 1,
            Plan::Business => 2,
            Plan::Agency => 3,
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
    fn catalog_prices_and_limits() {
        assert_eq!(Plan::Starter.spec().price_cents, 2900);
        assert_eq!(Plan::Starter.spec().annual_price_cents, 29000);
        assert_eq!(Plan::Starter.spec().backlink_limit, Some(500));
        assert_eq!(Plan::Starter.spec().project_limit, Some(3));
        assert_eq!(Plan::Starter.spec().seat_limit, Some(1));

        assert_eq!(Plan::Pro.spec().price_cents, 5900);
        assert_eq!(Plan::Pro.spec().backlink_limit, Some(2_500));
        assert_eq!(Plan::Pro.spec().project_limit, Some(10));
        assert_eq!(Plan::Pro.spec().seat_limit, Some(3));
        assert!(Plan::Pro.spec().most_popular, "Pro is the highlighted plan");

        assert_eq!(Plan::Business.spec().price_cents, 9900);
        assert_eq!(Plan::Business.spec().backlink_limit, Some(5_000));
        assert_eq!(Plan::Business.spec().project_limit, Some(25));
        assert_eq!(Plan::Business.spec().seat_limit, Some(5));

        assert_eq!(Plan::Agency.spec().price_cents, 19900);
        assert_eq!(Plan::Agency.spec().backlink_limit, Some(15_000));
        assert_eq!(Plan::Agency.spec().project_limit, Some(75));
        assert_eq!(Plan::Agency.spec().seat_limit, Some(15));
    }

    #[test]
    fn all_plans_monitor_every_48h() {
        for p in Plan::ALL {
            assert_eq!(p.spec().monitoring_interval_hours, 48);
        }
    }

    #[test]
    fn annual_is_about_two_months_free() {
        for p in Plan::ALL {
            let monthly_times_twelve = p.spec().price_cents * 12;
            assert!(
                p.spec().annual_price_cents < monthly_times_twelve,
                "annual must be cheaper than 12× monthly"
            );
            assert!(
                p.spec().annual_price_cents >= p.spec().price_cents * 10,
                "annual is roughly 10 months (≈ 2 months free)"
            );
        }
    }

    #[test]
    fn slugs_round_trip() {
        for p in Plan::ALL {
            assert_eq!(Plan::from_slug(p.spec().slug), Some(p));
        }
        assert_eq!(Plan::from_slug("agency_plus"), None, "old slug retired");
        assert_eq!(Plan::from_slug("enterprise"), None);
    }

    #[test]
    fn tiers_order_and_upgrade_direction() {
        assert!(Plan::Starter.is_upgrade_to(Plan::Pro));
        assert!(Plan::Pro.is_upgrade_to(Plan::Agency));
        assert!(!Plan::Agency.is_upgrade_to(Plan::Business));
        assert!(!Plan::Pro.is_upgrade_to(Plan::Pro));
    }

    #[test]
    fn price_increases_with_tier() {
        let mut last = 0;
        for p in Plan::ALL {
            assert!(p.spec().price_cents > last, "prices increase by tier");
            last = p.spec().price_cents;
        }
    }
}
