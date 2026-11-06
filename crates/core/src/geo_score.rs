//! GEO opportunity scorecard (Pillar 2 — "cited by AI"). Pure: citation signals
//! in, a 0–100 score plus its per-signal breakdown out — no I/O.
//!
//! This answers a different question from [`crate::spot_score`]: not "is this a
//! good site to get a link on?" but "how valuable is it that an AI assistant
//! cites this domain, and can we act on it?". It is the wedge of the new
//! positioning, so it stays **explainable** — every signal is 0–1 and shown to
//! the user, never a black box — and it **reuses** the Pillar 1+3 domain
//! scorecard for the underlying site quality instead of re-inventing it.
//!
//! Like every scorer in this crate, it is a prospecting aid, not a ranking
//! guarantee, and it **never fabricates** a citation: a missing signal lowers
//! the score, it does not invent presence (docs/product/21 §9).

use crate::spot_score::{SpotScore, SpotSignals, score as spot_score};

/// Inputs for one GEO-cited domain's score. The citation signals come from the
/// `CaptureGeo` pipeline (DataForSEO AI Overviews → parsed cited sources); the
/// `domain` block is the same authority/trust/traffic signals the spot
/// scorecard already reads, so the two pillars share one quality definition.
#[derive(Debug, Clone, Default)]
pub struct GeoSignals {
    /// How many of the project's tracked GEO queries cite this domain.
    pub citation_count: u32,
    /// How many GEO queries are tracked in total (the denominator). `0` means
    /// unknown — we then fall back to a log-saturation of `citation_count`.
    pub query_count: u32,
    /// Best (lowest) position the domain holds among an answer's cited sources,
    /// 1-based (`1` = first source the assistant cites). `None` if not captured.
    pub best_rank: Option<u32>,
    /// Distinct AI engines that cite the domain (AI Overview, Perplexity, …).
    pub engine_count: u32,
    /// Whether a buyable or open path to land a link on this domain exists
    /// (a matched marketplace offer or an open spot). An un-placeable citation
    /// is informational, not actionable, so it is down-weighted — not hidden.
    pub placeable: bool,
    /// The domain's underlying quality signals, scored by [`spot_score`].
    pub domain: SpotSignals,
}

/// The score and the normalized (0..1) signals behind it, for display.
#[derive(Debug, Clone, PartialEq)]
pub struct GeoScore {
    /// 0..=100, rounded to one decimal.
    pub total: f64,
    /// Share of tracked queries that cite the domain (AI presence).
    pub coverage: f64,
    /// How high the domain is cited within an answer (position quality).
    pub prominence: f64,
    /// Cross-engine reach (durable presence beats a single engine).
    pub reach: f64,
    /// Reused [`spot_score`] total, 0..1 — the site must be real and trafficked.
    pub quality: f64,
    /// Passed through for display: is there a path to actually place a link?
    pub placeable: bool,
}

// Signal weights (sum = 1.0). Coverage (how often AI cites it) and the reused
// domain quality lead; a citation on a dead site is worth little, and one we
// can't place on is worth less still (the `placeable` factor below).
const W_COVERAGE: f64 = 0.30;
const W_PROMINENCE: f64 = 0.25;
const W_REACH: f64 = 0.15;
const W_QUALITY: f64 = 0.30;

/// How many engines count as full cross-engine reach (AI Overview, Perplexity,
/// a third). Beyond this, reach is saturated.
const ENGINES_FOR_FULL_REACH: f64 = 3.0;
/// AI answers cite only a handful of sources; past this depth, prominence is 0.
const PROMINENCE_DEPTH: f64 = 8.0;
/// A cited domain with no placement path keeps this fraction of its score —
/// still surfaced (it is real AI visibility), but ranked below actionable ones.
const UNPLACEABLE_FACTOR: f64 = 0.6;

/// log10-based 0..1 saturation (same curve as the spot scorecard): `value`
/// reaches 1.0 at `full` and scales by the log of the ratio below it.
fn log_saturate(value: f64, full: f64) -> f64 {
    if value <= 0.0 {
        return 0.0;
    }
    if value >= full {
        return 1.0;
    }
    ((value + 1.0).ln() / (full + 1.0).ln()).clamp(0.0, 1.0)
}

/// Compute the explainable GEO opportunity score (0..100) and its breakdown.
pub fn score(s: &GeoSignals) -> GeoScore {
    // Coverage: share of tracked queries that cite the domain. When the
    // denominator is unknown (`query_count == 0`), saturate the raw count so a
    // domain cited for several queries still reads as broad AI presence.
    let coverage = if s.query_count > 0 {
        (s.citation_count as f64 / s.query_count as f64).clamp(0.0, 1.0)
    } else {
        log_saturate(s.citation_count as f64, 5.0)
    };

    // Prominence: being cited first is worth most; it decays with depth and is
    // 0 past PROMINENCE_DEPTH. No capture → 0 (we don't assume a good position).
    let prominence = match s.best_rank {
        Some(rank) if rank >= 1 => {
            (1.0 - (rank as f64 - 1.0) / PROMINENCE_DEPTH).clamp(0.0, 1.0)
        }
        _ => 0.0,
    };

    // Reach: more engines = more durable; saturates at ENGINES_FOR_FULL_REACH.
    let reach = (s.engine_count as f64 / ENGINES_FOR_FULL_REACH).clamp(0.0, 1.0);

    // Quality: reuse the Pillar 1+3 scorecard verbatim, normalized to 0..1.
    let SpotScore { total: q, .. } = spot_score(&s.domain);
    let quality = (q / 100.0).clamp(0.0, 1.0);

    let blended = W_COVERAGE * coverage
        + W_PROMINENCE * prominence
        + W_REACH * reach
        + W_QUALITY * quality;

    // Actionability gate: keep un-placeable citations, but rank them below ones
    // we can act on. Applied as a factor, not a weight, so it stays legible.
    let factor = if s.placeable { 1.0 } else { UNPLACEABLE_FACTOR };
    let total = blended * factor * 100.0;

    GeoScore {
        total: (total * 10.0).round() / 10.0,
        coverage,
        prominence,
        reach,
        quality,
        placeable: s.placeable,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strong_domain() -> SpotSignals {
        SpotSignals {
            keyword_count: 6,
            domain_rank: Some(800),
            referring_domains: Some(4000),
            spam_score: Some(3),
            organic_keywords: Some(15_000),
            organic_traffic: Some(200_000),
        }
    }

    #[test]
    fn empty_signals_score_near_zero() {
        // No citations, no rank, no engines; only the neutral-trust slice of the
        // reused quality (default SpotSignals → 7.5/100) feeds through.
        let s = score(&GeoSignals::default());
        // quality 0.075 * W_QUALITY 0.30 * UNPLACEABLE_FACTOR 0.6 * 100 ≈ 1.35
        assert!(s.total < 2.0, "got {}", s.total);
        assert_eq!(s.coverage, 0.0);
        assert_eq!(s.prominence, 0.0);
        assert_eq!(s.reach, 0.0);
    }

    #[test]
    fn well_cited_placeable_domain_scores_high() {
        let s = score(&GeoSignals {
            citation_count: 8,
            query_count: 10,
            best_rank: Some(1),
            engine_count: 3,
            placeable: true,
            domain: strong_domain(),
        });
        assert!(s.total > 85.0, "got {}", s.total);
        assert_eq!(s.prominence, 1.0); // rank 1 is maximal
        assert_eq!(s.reach, 1.0); // 3 engines saturates
        assert!((s.coverage - 0.8).abs() < 1e-9); // 8/10
        assert!(s.quality > 0.8);
    }

    #[test]
    fn coverage_uses_denominator_when_known() {
        let half = score(&GeoSignals {
            citation_count: 5,
            query_count: 10,
            ..Default::default()
        });
        assert!((half.coverage - 0.5).abs() < 1e-9);
        // More citations over the same denominator ⇒ higher coverage.
        let most = score(&GeoSignals {
            citation_count: 9,
            query_count: 10,
            ..Default::default()
        });
        assert!(most.coverage > half.coverage);
    }

    #[test]
    fn coverage_falls_back_to_saturation_without_denominator() {
        let s = score(&GeoSignals {
            citation_count: 5,
            query_count: 0,
            ..Default::default()
        });
        assert_eq!(s.coverage, 1.0); // 5 citations saturates the fallback curve
    }

    #[test]
    fn prominence_decays_with_rank_and_floors_at_depth() {
        let r1 = score(&GeoSignals {
            best_rank: Some(1),
            ..Default::default()
        })
        .prominence;
        let r4 = score(&GeoSignals {
            best_rank: Some(4),
            ..Default::default()
        })
        .prominence;
        let r20 = score(&GeoSignals {
            best_rank: Some(20),
            ..Default::default()
        })
        .prominence;
        assert_eq!(r1, 1.0);
        assert!(r4 < r1 && r4 > 0.0);
        assert_eq!(r20, 0.0); // past PROMINENCE_DEPTH
    }

    #[test]
    fn unplaceable_citation_scores_below_identical_placeable_one() {
        let base = GeoSignals {
            citation_count: 6,
            query_count: 10,
            best_rank: Some(2),
            engine_count: 2,
            placeable: true,
            domain: strong_domain(),
        };
        let placeable = score(&base);
        let unplaceable = score(&GeoSignals {
            placeable: false,
            ..base
        });
        assert!(unplaceable.total < placeable.total);
        // Exactly the UNPLACEABLE_FACTOR down-weight, within rounding.
        assert!(
            (unplaceable.total - placeable.total * UNPLACEABLE_FACTOR).abs() < 0.2,
            "{} vs {}",
            unplaceable.total,
            placeable.total
        );
        // But it is NOT hidden — still a positive, surfaced score.
        assert!(unplaceable.total > 0.0);
    }

    #[test]
    fn quality_reuses_spot_score() {
        // Same citation signals, different underlying domain ⇒ quality moves the
        // total, proving the spot scorecard is actually wired in.
        let common = GeoSignals {
            citation_count: 5,
            query_count: 10,
            best_rank: Some(3),
            engine_count: 2,
            placeable: true,
            ..Default::default()
        };
        let weak = score(&GeoSignals {
            domain: SpotSignals {
                spam_score: Some(95),
                ..Default::default()
            },
            ..common.clone()
        });
        let strong = score(&GeoSignals {
            domain: strong_domain(),
            ..common
        });
        assert!(strong.quality > weak.quality);
        assert!(strong.total > weak.total);
    }

    #[test]
    fn deterministic_and_bounded() {
        let s = GeoSignals {
            citation_count: 7,
            query_count: 9,
            best_rank: Some(2),
            engine_count: 2,
            placeable: true,
            domain: strong_domain(),
        };
        assert_eq!(score(&s), score(&s));
        assert!(score(&s).total >= 0.0 && score(&s).total <= 100.0);
    }
}
