//! Prospect scoring (link-building campaigns, BETA).
//!
//! A prospect's score is an *explainable* priority: given only local signals
//! (how many of our competitors already link to the domain, whether we have a
//! contact, whether a cost is estimated, and the pipeline stage), produce a
//! 0–100 score plus the human-readable reasons behind it. Like the monitoring
//! policy, this is the ONE place the ranking is decided — pure, no I/O, no clock,
//! so it is exhaustively unit-tested and the UI and the data layer agree.
//!
//! This is also the seam for a smarter model later (authority metrics, topical
//! relevance, an LLM): callers pass signals and read back {score, reasons}, so
//! the inside can change without touching the API or UI.

/// The local signals the scorer reads about a prospect.
#[derive(Debug, Clone, Copy)]
pub struct ProspectSignals<'a> {
    /// How many of the project's competitors link to this domain (from the gap
    /// analysis). More competitors ⇒ more proven, reachable value.
    pub competitor_count: i32,
    /// Whether we have a contact (name or email) to reach out to.
    pub has_contact: bool,
    /// Whether an acquisition cost has been estimated.
    pub has_estimated_cost: bool,
    /// Pipeline stage slug (`identified`..`won`).
    pub stage: &'a str,
}

/// The scorer's verdict: a 0–100 priority and the reasons for it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProspectScore {
    pub score: u8,
    pub reasons: Vec<&'static str>,
}

/// How much each proven competitor link is worth, and the cap on that component.
const PER_COMPETITOR: i32 = 14;
const MAX_COMPETITORS_SCORED: i32 = 5; // up to 70 points from competitor overlap

/// Score a prospect from its local signals. Deterministic; higher = pursue
/// sooner. A won prospect is pinned to 100 (it is already a result).
pub fn score_prospect(s: ProspectSignals<'_>) -> ProspectScore {
    if s.stage == "won" {
        return ProspectScore {
            score: 100,
            reasons: vec!["already won"],
        };
    }
    if s.stage == "lost" {
        return ProspectScore {
            score: 0,
            reasons: vec!["marked lost"],
        };
    }

    let mut score: i32 = 0;
    let mut reasons: Vec<&'static str> = Vec::new();

    let c = s.competitor_count.clamp(0, MAX_COMPETITORS_SCORED);
    if c > 0 {
        score += c * PER_COMPETITOR;
        reasons.push("links from competitors");
    }
    if s.has_contact {
        score += 15;
        reasons.push("contact on file");
    }
    if s.has_estimated_cost {
        score += 10;
        reasons.push("cost estimated");
    }
    // Momentum: a prospect we have engaged is closer to a result.
    match s.stage {
        "qualified" => {
            score += 5;
            reasons.push("qualified");
        }
        "contacted" => {
            score += 8;
            reasons.push("outreach sent");
        }
        "negotiating" => {
            score += 12;
            reasons.push("in negotiation");
        }
        _ => {}
    }
    if reasons.is_empty() {
        reasons.push("no strong signals yet");
    }

    ProspectScore {
        score: score.clamp(0, 100) as u8,
        reasons,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sig(competitor_count: i32) -> ProspectSignals<'static> {
        ProspectSignals {
            competitor_count,
            has_contact: false,
            has_estimated_cost: false,
            stage: "identified",
        }
    }

    #[test]
    fn more_competitors_scores_higher() {
        assert!(score_prospect(sig(3)).score > score_prospect(sig(1)).score);
        assert!(score_prospect(sig(1)).score > score_prospect(sig(0)).score);
    }

    #[test]
    fn competitor_component_is_capped() {
        // Beyond the cap, extra competitors add nothing.
        assert_eq!(score_prospect(sig(5)).score, score_prospect(sig(50)).score);
    }

    #[test]
    fn contact_and_cost_are_additive_bonuses() {
        let base = score_prospect(sig(2)).score;
        let mut s = sig(2);
        s.has_contact = true;
        let with_contact = score_prospect(s).score;
        s.has_estimated_cost = true;
        let with_both = score_prospect(s).score;
        assert_eq!(with_contact, base + 15);
        assert_eq!(with_both, base + 15 + 10);
    }

    #[test]
    fn won_is_pinned_to_100_and_lost_to_0() {
        let mut s = sig(0);
        s.stage = "won";
        assert_eq!(score_prospect(s).score, 100);
        s.stage = "lost";
        assert_eq!(score_prospect(s).score, 0);
    }

    #[test]
    fn deterministic() {
        let mut s = sig(3);
        s.has_contact = true;
        s.stage = "negotiating";
        assert_eq!(score_prospect(s), score_prospect(s));
    }

    #[test]
    fn empty_signals_still_explained() {
        assert_eq!(
            score_prospect(sig(0)).reasons,
            vec!["no strong signals yet"]
        );
        assert_eq!(score_prospect(sig(0)).score, 0);
    }
}
