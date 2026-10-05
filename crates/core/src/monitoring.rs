//! Monitoring eligibility policy (selective monitoring, cost-control).
//!
//! Not every known backlink should be actively re-checked: monitoring has a real
//! crawler cost, so we only monitor links with an economic, contractual, or
//! strategic reason — or ones the user explicitly chose. This module is the ONE
//! place that decides the DEFAULT monitoring stance for a link; everything else
//! (scheduler, create paths, the data migration) follows it. Pure, no I/O, no
//! clock, so the rules are exhaustively unit-tested.
//!
//! A user's manual choice always wins: callers skip the policy for a link whose
//! `monitoring_overridden` flag is set.

/// How actively a backlink is monitored.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MonitoringLevel {
    /// Highest care — monitored, and represents protected investment/value.
    Protected,
    /// Monitored, but lower stakes than protected.
    Tracked,
    /// Known but NOT monitored — generates zero recurring crawl.
    InventoryOnly,
}

impl MonitoringLevel {
    pub fn as_str(self) -> &'static str {
        match self {
            MonitoringLevel::Protected => "protected",
            MonitoringLevel::Tracked => "tracked",
            MonitoringLevel::InventoryOnly => "inventory_only",
        }
    }
}

/// Why a link is monitored (surfaced in the UI; stored as text).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MonitoringReason {
    EconomicValue,
    PaidLink,
    Exchange,
    Partnership,
    Guarantee,
    Strategic,
    Manual,
}

impl MonitoringReason {
    pub fn as_str(self) -> &'static str {
        match self {
            MonitoringReason::EconomicValue => "economic_value",
            MonitoringReason::PaidLink => "paid_link",
            MonitoringReason::Exchange => "exchange",
            MonitoringReason::Partnership => "partnership",
            MonitoringReason::Guarantee => "guarantee",
            MonitoringReason::Strategic => "strategic",
            MonitoringReason::Manual => "manual",
        }
    }
}

/// The signals the policy reads about a link.
#[derive(Debug, Clone, Copy)]
pub struct MonitoringInputs<'a> {
    /// Acquisition classification slug (migration 0034), if set.
    pub acquisition: Option<&'a str>,
    /// Whether the link has a positive acquisition cost.
    pub has_cost: bool,
    /// Whether the link carries a warranty/guarantee (warranty_type != none).
    pub under_warranty: bool,
    /// "own" | "competitor".
    pub subject: &'a str,
    /// Graph tier: 1 = direct, 2+ = reinforcing.
    pub tier: i16,
}

/// The policy's default decision for a link.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MonitoringDecision {
    pub level: MonitoringLevel,
    pub enabled: bool,
    pub reason: Option<MonitoringReason>,
}

impl MonitoringDecision {
    fn inventory() -> Self {
        MonitoringDecision {
            level: MonitoringLevel::InventoryOnly,
            enabled: false,
            reason: None,
        }
    }
    fn monitored(level: MonitoringLevel, reason: MonitoringReason) -> Self {
        MonitoringDecision {
            level,
            enabled: true,
            reason: Some(reason),
        }
    }
}

/// Decide the DEFAULT monitoring stance for a link. First matching rule wins:
/// 1. competitor links and tier > 1 are never monitored by default (cost);
/// 2. a cost or a warranty is an economic/contractual reason -> protected;
/// 3. acquisition method: paid -> protected, exchange/partnership -> tracked;
/// 4. everything else (organic/earned/self-created/unclassified) -> inventory.
pub fn decide(i: MonitoringInputs<'_>) -> MonitoringDecision {
    if i.subject != "own" || i.tier != 1 {
        return MonitoringDecision::inventory();
    }
    if i.has_cost {
        return MonitoringDecision::monitored(
            MonitoringLevel::Protected,
            MonitoringReason::EconomicValue,
        );
    }
    if i.under_warranty {
        return MonitoringDecision::monitored(
            MonitoringLevel::Protected,
            MonitoringReason::Guarantee,
        );
    }
    match i.acquisition {
        Some("paid") => {
            MonitoringDecision::monitored(MonitoringLevel::Protected, MonitoringReason::PaidLink)
        }
        Some("exchange") => {
            MonitoringDecision::monitored(MonitoringLevel::Tracked, MonitoringReason::Exchange)
        }
        Some("partnership") => {
            MonitoringDecision::monitored(MonitoringLevel::Tracked, MonitoringReason::Partnership)
        }
        _ => MonitoringDecision::inventory(),
    }
}

/// The recurring interval (in days) for a monitored link's `check_frequency`.
/// `None` means "no automatic schedule" — either an unknown value or `manual`
/// (the user triggers checks on demand); `weekly`/default is 7 days, matching
/// the state machine's base interval.
pub fn frequency_days(freq: Option<&str>) -> Option<i64> {
    match freq {
        Some("daily") => Some(1),
        Some("biweekly") => Some(14),
        Some("monthly") => Some(30),
        Some("manual") => None,
        // "weekly", None, or anything unexpected -> the 7-day default cadence.
        _ => Some(7),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn own(acquisition: Option<&str>) -> MonitoringInputs<'_> {
        MonitoringInputs {
            acquisition,
            has_cost: false,
            under_warranty: false,
            subject: "own",
            tier: 1,
        }
    }

    #[test]
    fn paid_is_protected() {
        let d = decide(own(Some("paid")));
        assert!(d.enabled);
        assert_eq!(d.level, MonitoringLevel::Protected);
        assert_eq!(d.reason, Some(MonitoringReason::PaidLink));
    }

    #[test]
    fn organic_is_inventory() {
        for a in [
            Some("earned"),
            Some("self_created"),
            Some("outreach"),
            Some("other"),
            None,
        ] {
            let d = decide(own(a));
            assert!(!d.enabled, "{a:?} should be inventory");
            assert_eq!(d.level, MonitoringLevel::InventoryOnly);
            assert_eq!(d.reason, None);
        }
    }

    #[test]
    fn cost_forces_monitoring_regardless_of_acquisition() {
        let mut i = own(Some("earned"));
        i.has_cost = true;
        let d = decide(i);
        assert!(d.enabled);
        assert_eq!(d.level, MonitoringLevel::Protected);
        assert_eq!(d.reason, Some(MonitoringReason::EconomicValue));
    }

    #[test]
    fn warranty_forces_monitoring() {
        let mut i = own(None);
        i.under_warranty = true;
        let d = decide(i);
        assert!(d.enabled);
        assert_eq!(d.reason, Some(MonitoringReason::Guarantee));
    }

    #[test]
    fn exchange_and_partnership_are_tracked() {
        assert_eq!(
            decide(own(Some("exchange"))).level,
            MonitoringLevel::Tracked
        );
        assert_eq!(
            decide(own(Some("partnership"))).level,
            MonitoringLevel::Tracked
        );
    }

    #[test]
    fn competitor_and_tier2_are_never_monitored_by_default() {
        let mut comp = own(Some("paid"));
        comp.subject = "competitor";
        assert!(!decide(comp).enabled, "competitor paid still inventory");

        let mut t2 = own(Some("paid"));
        t2.has_cost = true;
        t2.tier = 2;
        assert!(!decide(t2).enabled, "tier-2 with cost still inventory");
    }

    #[test]
    fn frequency_mapping() {
        assert_eq!(frequency_days(Some("daily")), Some(1));
        assert_eq!(frequency_days(Some("weekly")), Some(7));
        assert_eq!(frequency_days(Some("biweekly")), Some(14));
        assert_eq!(frequency_days(Some("monthly")), Some(30));
        assert_eq!(frequency_days(Some("manual")), None);
        assert_eq!(frequency_days(None), Some(7));
    }
}
