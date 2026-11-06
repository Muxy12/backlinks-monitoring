//! Traffic health & decline detection (Pillar 3 — "real living traffic").
//! Pure: two metric samples in, a health flag plus its reasons out — no I/O,
//! no clock.
//!
//! The promise of Pillar 3 is that we only surface sites with *real, living*
//! traffic — not a domain that is quietly dying, and not one whose traffic
//! number is implausible on its face. This is the one place that judgement is
//! made, so it is exhaustively tested and shared by the scorer, the
//! `RefreshSpotTrends` job, and the UI.
//!
//! It is deliberately conservative: it flags **decline** only on a corroborated
//! or large drop, and **suspect** only on an internally inconsistent snapshot
//! (traffic with almost no ranked keywords or referring domains). A thin sample
//! is reported as `Unknown`, never guessed — we do not fabricate a trend
//! (docs/product/21 §9).

/// A point-in-time metrics snapshot for a domain (a subset of the crawler's
/// `DomainMetrics`). Each field is best-effort; `None` = the provider didn't
/// return it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TrafficSample {
    /// Estimated monthly organic traffic.
    pub organic_traffic: Option<i64>,
    /// Keywords the domain ranks for across Google.
    pub organic_keywords: Option<i32>,
    /// Referring domains pointing at the site.
    pub referring_domains: Option<i32>,
}

/// The health verdict for a domain's trend.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealthFlag {
    /// Stable or growing — a real, living site.
    Healthy,
    /// A meaningful, corroborated drop in traffic/keywords.
    Declining,
    /// Internally inconsistent snapshot (traffic without the keywords or links
    /// that would produce it) — a possible fake-traffic signal.
    Suspect,
    /// Not enough data to judge.
    Unknown,
}

/// The assessment: the flag, the computed traffic change (when both samples
/// have traffic), and the human-readable reasons behind it.
#[derive(Debug, Clone, PartialEq)]
pub struct DeclineAssessment {
    pub flag: HealthFlag,
    /// `(current - prior) / prior` as a fraction, e.g. `-0.4` = down 40%.
    /// `None` when either sample lacks traffic.
    pub traffic_change: Option<f64>,
    pub reasons: Vec<&'static str>,
}

// --- Thresholds (documented, tunable in one place) -------------------------

/// A drop this large on its own is a decline, even without corroboration.
const DECLINE_ALONE: f64 = -0.50;
/// A smaller drop counts as decline only if keywords also fell.
const DECLINE_CORROBORATED: f64 = -0.30;
/// Keyword drop that corroborates a traffic decline.
const KEYWORDS_DROP_CORROBORATING: f64 = -0.20;
/// Below this traffic we don't bother judging plausibility (too small to tell).
const SUSPECT_MIN_TRAFFIC: i64 = 1_000;
/// Traffic above `SUSPECT_MIN_TRAFFIC` with keywords at or below this is
/// implausible (real organic traffic needs ranked keywords).
const SUSPECT_MAX_KEYWORDS: i32 = 10;

/// Assess a domain's health from a `prior` and a `current` sample. When no
/// prior exists, pass `TrafficSample::default()` — the plausibility
/// (suspect / unknown) checks on `current` still apply.
pub fn assess(prior: &TrafficSample, current: &TrafficSample) -> DeclineAssessment {
    let mut reasons: Vec<&'static str> = Vec::new();

    // Plausibility first: an internally inconsistent *current* snapshot is
    // suspect regardless of any trend (this also catches first-run, no-prior).
    if is_suspect(current) {
        reasons.push("traffic implausible for ranked keywords / backlinks");
        return DeclineAssessment {
            flag: HealthFlag::Suspect,
            traffic_change: traffic_change(prior, current),
            reasons,
        };
    }

    let change = traffic_change(prior, current);
    let Some(change) = change else {
        // Can't compute a trend (missing traffic on one side).
        reasons.push("insufficient data for a trend");
        return DeclineAssessment {
            flag: HealthFlag::Unknown,
            traffic_change: None,
            reasons,
        };
    };

    let kw_change = pct_change(
        prior.organic_keywords.map(|k| k as f64),
        current.organic_keywords.map(|k| k as f64),
    );
    let keywords_down = kw_change.is_some_and(|c| c <= KEYWORDS_DROP_CORROBORATING);

    let declining = change <= DECLINE_ALONE
        || (change <= DECLINE_CORROBORATED && keywords_down);

    if declining {
        reasons.push("organic traffic falling");
        if keywords_down {
            reasons.push("ranked keywords also falling");
        }
        DeclineAssessment {
            flag: HealthFlag::Declining,
            traffic_change: Some(change),
            reasons,
        }
    } else {
        reasons.push(if change >= 0.0 {
            "traffic stable or growing"
        } else {
            "traffic dip within normal range"
        });
        DeclineAssessment {
            flag: HealthFlag::Healthy,
            traffic_change: Some(change),
            reasons,
        }
    }
}

/// Convenience boolean used by `RefreshSpotTrends` and the spot list: `true`
/// iff the domain is in decline. `Suspect`/`Unknown`/`Healthy` are all `false`
/// (suspect is a separate axis; see [`is_suspect`]).
pub fn decline_flag(prior: &TrafficSample, current: &TrafficSample) -> bool {
    assess(prior, current).flag == HealthFlag::Declining
}

/// Whether a single snapshot is internally inconsistent (traffic above the
/// floor but essentially no ranked keywords *and* no referring domains). Real
/// organic traffic is produced by ranked keywords and earned links; traffic
/// without either is a classic bought-/bot-traffic footprint.
pub fn is_suspect(s: &TrafficSample) -> bool {
    let Some(traffic) = s.organic_traffic else {
        return false;
    };
    if traffic < SUSPECT_MIN_TRAFFIC {
        return false;
    }
    let keywords_thin = s.organic_keywords.is_some_and(|k| k <= SUSPECT_MAX_KEYWORDS);
    let links_thin = s.referring_domains.is_some_and(|r| r == 0);
    // Require a thin keyword signal; the thin-links signal corroborates it. We
    // only assert "suspect" when at least the keyword side is both present and
    // implausible, so an unmeasured domain is never accused.
    keywords_thin && (links_thin || s.referring_domains.is_none())
}

/// `(current - prior) / prior` fraction, or `None` if a side is missing or the
/// prior is zero.
fn traffic_change(prior: &TrafficSample, current: &TrafficSample) -> Option<f64> {
    pct_change(
        prior.organic_traffic.map(|t| t as f64),
        current.organic_traffic.map(|t| t as f64),
    )
}

fn pct_change(prior: Option<f64>, current: Option<f64>) -> Option<f64> {
    match (prior, current) {
        (Some(p), Some(c)) if p > 0.0 => Some((c - p) / p),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(traffic: i64, keywords: i32, refd: i32) -> TrafficSample {
        TrafficSample {
            organic_traffic: Some(traffic),
            organic_keywords: Some(keywords),
            referring_domains: Some(refd),
        }
    }

    #[test]
    fn stable_traffic_is_healthy() {
        let a = assess(&sample(10_000, 500, 300), &sample(10_200, 510, 300));
        assert_eq!(a.flag, HealthFlag::Healthy);
        assert!(a.traffic_change.unwrap() > 0.0);
        assert!(!decline_flag(&sample(10_000, 500, 300), &sample(10_200, 510, 300)));
    }

    #[test]
    fn large_drop_alone_is_decline() {
        // Traffic halved, keywords flat — the drop is large enough on its own.
        let prior = sample(10_000, 500, 300);
        let current = sample(4_000, 500, 300);
        let a = assess(&prior, &current);
        assert_eq!(a.flag, HealthFlag::Declining);
        assert!(decline_flag(&prior, &current));
        assert!((a.traffic_change.unwrap() - -0.6).abs() < 1e-9);
    }

    #[test]
    fn moderate_drop_needs_keyword_corroboration() {
        // -35% traffic but keywords steady ⇒ not yet a decline.
        let steady_kw = assess(&sample(10_000, 500, 300), &sample(6_500, 500, 300));
        assert_eq!(steady_kw.flag, HealthFlag::Healthy);
        // Same traffic drop with keywords also down ⇒ decline.
        let kw_down = assess(&sample(10_000, 500, 300), &sample(6_500, 350, 300));
        assert_eq!(kw_down.flag, HealthFlag::Declining);
        assert!(kw_down.reasons.contains(&"ranked keywords also falling"));
    }

    #[test]
    fn traffic_without_keywords_or_links_is_suspect() {
        // High traffic, ~no keywords, no referring domains → fake-traffic shape.
        let s = sample(50_000, 2, 0);
        assert!(is_suspect(&s));
        let a = assess(&TrafficSample::default(), &s);
        assert_eq!(a.flag, HealthFlag::Suspect);
    }

    #[test]
    fn small_traffic_is_not_judged_suspect() {
        // Below the floor we don't accuse — too small to tell.
        assert!(!is_suspect(&sample(200, 1, 0)));
    }

    #[test]
    fn unmeasured_domain_is_never_suspect() {
        // Traffic present but keywords unknown ⇒ no accusation.
        let s = TrafficSample {
            organic_traffic: Some(50_000),
            organic_keywords: None,
            referring_domains: None,
        };
        assert!(!is_suspect(&s));
    }

    #[test]
    fn missing_prior_traffic_is_unknown() {
        let a = assess(&TrafficSample::default(), &sample(10_000, 500, 300));
        assert_eq!(a.flag, HealthFlag::Unknown);
        assert_eq!(a.traffic_change, None);
    }

    #[test]
    fn zero_prior_traffic_does_not_divide_by_zero() {
        let a = assess(&sample(0, 0, 0), &sample(5_000, 200, 100));
        // prior traffic 0 → change uncomputable → Unknown (current is plausible).
        assert_eq!(a.flag, HealthFlag::Unknown);
    }

    #[test]
    fn suspect_takes_precedence_over_trend() {
        // Even a growing-traffic snapshot is suspect if it's implausible.
        let prior = sample(1_000, 5, 0);
        let current = sample(80_000, 3, 0);
        assert_eq!(assess(&prior, &current).flag, HealthFlag::Suspect);
    }

    #[test]
    fn deterministic() {
        let prior = sample(10_000, 500, 300);
        let current = sample(4_000, 300, 300);
        assert_eq!(assess(&prior, &current), assess(&prior, &current));
    }
}
