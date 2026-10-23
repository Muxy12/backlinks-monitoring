//! Spot quality scorecard (Spot Finder, Lot C). Pure: metrics in, a 0–100 score
//! plus its per-signal breakdown out — no I/O.
//!
//! This is an **explainable prospecting aid**, deliberately not a black box and
//! not a ranking guarantee. Every signal is 0–1 and shown to the user; the total
//! is a weighted blend. Relevance (how many of the user's keywords the domain
//! ranks for) is weighted highest — a topically-relevant site is the point.

/// Inputs for one spot's score. All optional except relevance (which comes from
/// the SERP stage); a missing metric contributes 0 to its signal.
#[derive(Debug, Clone, Default)]
pub struct SpotSignals {
    /// How many of the user's keywords this domain ranks for (from the SERP run).
    pub keyword_count: u32,
    /// DataForSEO domain rank, 0..=1000 (authority).
    pub domain_rank: Option<i32>,
    /// Referring domains pointing at the site.
    pub referring_domains: Option<i32>,
    /// Backlink spam score 0..=100 (lower is better).
    pub spam_score: Option<i32>,
    /// Keywords the domain ranks for across all of Google (breadth).
    pub organic_keywords: Option<i32>,
    /// Estimated monthly organic traffic.
    pub organic_traffic: Option<i64>,
}

/// The score and the normalized (0..1) signals behind it, for display.
#[derive(Debug, Clone, PartialEq)]
pub struct SpotScore {
    /// 0..=100, rounded to one decimal.
    pub total: f64,
    pub relevance: f64,
    pub authority: f64,
    pub trust: f64,
    pub traffic: f64,
    pub breadth: f64,
}

// Signal weights (sum = 1.0). Relevance leads by design.
const W_RELEVANCE: f64 = 0.35;
const W_AUTHORITY: f64 = 0.25;
const W_TRUST: f64 = 0.15;
const W_TRAFFIC: f64 = 0.15;
const W_BREADTH: f64 = 0.10;

/// log10-based 0..1 saturation: `value` maps to 1.0 at `full` and scales by the
/// log of the ratio below it, so big numbers don't dwarf everything.
fn log_saturate(value: f64, full: f64) -> f64 {
    if value <= 0.0 {
        return 0.0;
    }
    if value >= full {
        return 1.0;
    }
    ((value + 1.0).ln() / (full + 1.0).ln()).clamp(0.0, 1.0)
}

/// Compute the explainable quality score (0..100) and its signal breakdown.
pub fn score(s: &SpotSignals) -> SpotScore {
    // Relevance: ranking for ~5 of the user's keywords is a strong signal.
    let relevance = log_saturate(s.keyword_count as f64, 5.0);
    // Authority: domain rank 0..1000 → 0..1.
    let authority = s
        .domain_rank
        .map(|r| (r as f64 / 1000.0).clamp(0.0, 1.0))
        .unwrap_or(0.0);
    // Trust: a low spam score is good. Unknown spam → neutral 0.5 (we don't
    // punish a site just because we couldn't measure it).
    let trust = match s.spam_score {
        Some(spam) => (1.0 - (spam as f64 / 100.0)).clamp(0.0, 1.0),
        None => 0.5,
    };
    // Traffic: saturates around 100k est. monthly visits.
    let traffic = s
        .organic_traffic
        .map(|t| log_saturate(t as f64, 100_000.0))
        .unwrap_or(0.0);
    // Breadth: saturates around 10k ranked keywords.
    let breadth = s
        .organic_keywords
        .map(|k| log_saturate(k as f64, 10_000.0))
        .unwrap_or(0.0);

    let total = (W_RELEVANCE * relevance
        + W_AUTHORITY * authority
        + W_TRUST * trust
        + W_TRAFFIC * traffic
        + W_BREADTH * breadth)
        * 100.0;

    SpotScore {
        total: (total * 10.0).round() / 10.0,
        relevance,
        authority,
        trust,
        traffic,
        breadth,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_signals_score_low_but_not_zero_trust() {
        // Only the neutral trust (0.5 * 0.15) contributes.
        let s = score(&SpotSignals::default());
        assert!((s.total - 7.5).abs() < 0.01, "got {}", s.total);
        assert_eq!(s.trust, 0.5);
        assert_eq!(s.relevance, 0.0);
    }

    #[test]
    fn strong_spot_scores_high() {
        let s = score(&SpotSignals {
            keyword_count: 6,
            domain_rank: Some(850),
            referring_domains: Some(5000),
            spam_score: Some(2),
            organic_keywords: Some(20_000),
            organic_traffic: Some(250_000),
        });
        assert!(s.total > 85.0, "got {}", s.total);
        assert_eq!(s.relevance, 1.0); // ≥5 keywords saturates
        assert!(s.authority > 0.8);
        assert!(s.trust > 0.95);
    }

    #[test]
    fn spam_lowers_trust() {
        let clean = score(&SpotSignals {
            spam_score: Some(0),
            ..Default::default()
        });
        let spammy = score(&SpotSignals {
            spam_score: Some(90),
            ..Default::default()
        });
        assert!(clean.trust > spammy.trust);
        assert!(clean.total > spammy.total);
    }

    #[test]
    fn relevance_dominates_between_two_otherwise_equal_spots() {
        let few = score(&SpotSignals {
            keyword_count: 1,
            domain_rank: Some(400),
            ..Default::default()
        });
        let many = score(&SpotSignals {
            keyword_count: 5,
            domain_rank: Some(400),
            ..Default::default()
        });
        assert!(many.total > few.total);
    }
}
