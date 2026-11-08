//! Thematic-performance scores (Spot Finder, Netlinking view). Pure: a spot's
//! Google positions on the project's keywords in, two explainable 0–100 scores
//! out, plus a relative price/performance grade computed across the whole set.
//!
//! The scores are deliberately built from REAL ranking positions, not from a
//! third-party, manipulable metric (Trust Flow, referring-domain counts…). A
//! site can inflate those; it cannot fake ranking in your thematic.
//!
//! - **PTA — current thematic performance**: how much the spot already weighs
//!   across your WHOLE thematic. Averaged over EVERY keyword that maps your
//!   thematic, including the ones it does not rank for (they count as zero), so
//!   broad coverage is rewarded. This is the score that orders the ranking.
//! - **PTP — potential thematic performance**: the quality of its positions only
//!   WHERE it ranks. A narrow-but-excellent spot scores low PTA, high PTP.
//!
//! 100 ≈ ranked #1 on every one of your keywords (nobody reaches it; it anchors
//! the scale).

/// Value of one organic position, 1.0 at rank 1 and decaying with depth. A
/// smooth, explainable CTR-like proxy (`1/√rank`): rank 1 → 1.0, 4 → 0.5,
/// 10 → 0.32, 20 → 0.22, 100 → 0.1.
fn position_value(rank: i32) -> f64 {
    if rank < 1 {
        return 0.0;
    }
    1.0 / (rank as f64).sqrt()
}

fn round1(x: f64) -> f64 {
    (x * 10.0).round() / 10.0
}

/// PTA and PTP for one spot, 0..=100 each.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ThematicScore {
    /// Current thematic performance (weighs across all keywords). Orders the rank.
    pub current: f64,
    /// Potential thematic performance (quality where it ranks).
    pub potential: f64,
}

/// Compute PTA/PTP from a spot's organic positions (one per keyword it ranks
/// for) and the total number of keywords mapping the thematic. Positions < 1 are
/// ignored. With no total keywords or no ranked positions the scores are 0.
pub fn thematic_score(positions: &[i32], total_keywords: u32) -> ThematicScore {
    let values: Vec<f64> = positions
        .iter()
        .copied()
        .filter(|&p| p >= 1)
        .map(position_value)
        .collect();
    let sum: f64 = values.iter().sum();
    let current = if total_keywords > 0 {
        (100.0 * sum / total_keywords as f64).clamp(0.0, 100.0)
    } else {
        0.0
    };
    let potential = if values.is_empty() {
        0.0
    } else {
        (100.0 * sum / values.len() as f64).clamp(0.0, 100.0)
    };
    ThematicScore {
        current: round1(current),
        potential: round1(potential),
    }
}

/// The price/performance grade of a spot, relative to the set it belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Grade {
    /// Top 2% price/perf of the ranking (and clears the A+ floor).
    Pepite,
    /// Best quartile of price/perf, floor cleared.
    APlus,
    /// Between the best quartile and the median.
    A,
    /// Between the median and the worst quartile.
    B,
    /// Worst quartile — most expensive for its measured performance.
    C,
    /// No price or no measured performance → not graded.
    Unrated,
}

impl Grade {
    /// Stable machine label (for the API / storage).
    pub fn as_str(self) -> &'static str {
        match self {
            Grade::Pepite => "pepite",
            Grade::APlus => "a_plus",
            Grade::A => "a",
            Grade::B => "b",
            Grade::C => "c",
            Grade::Unrated => "unrated",
        }
    }
}

/// One spot's inputs for the relative price/performance grade.
#[derive(Debug, Clone, Copy)]
pub struct SpotGradeInput {
    pub current: f64,
    pub potential: f64,
    /// Cheapest placement price across marketplaces, in cents. `None` → unrated.
    pub price_cents: Option<i64>,
    /// Distinct ranking pages on the spot's domain (one lever of the A+ floor).
    pub distinct_pages: u32,
}

/// Grade a whole set of spots by price ÷ current-performance, by quartiles
/// (lower ratio = better value), with the top 2% flagged `Pepite`. Reaching the
/// best quartile (A+/Pepite) also requires a quality floor — at least 3 distinct
/// ranking pages OR current ≥ 60, AND potential ≥ 40 — else it drops to A. Spots
/// with no price or no performance are `Unrated` and excluded from the ranking.
/// Returns one grade per input, in the same order.
pub fn grade_set(spots: &[SpotGradeInput]) -> Vec<Grade> {
    let mut out = vec![Grade::Unrated; spots.len()];

    // Rated = has a positive price and a positive current score. Rank by ratio.
    let mut rated: Vec<(usize, f64)> = spots
        .iter()
        .enumerate()
        .filter_map(|(i, s)| match s.price_cents {
            Some(c) if c > 0 && s.current > 0.0 => Some((i, c as f64 / s.current)),
            _ => None,
        })
        .collect();
    let m = rated.len();
    if m == 0 {
        return out;
    }
    rated.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

    for (rank, &(i, _ratio)) in rated.iter().enumerate() {
        let pct = rank as f64 / m as f64;
        let s = &spots[i];
        let floor_ok = (s.distinct_pages >= 3 || s.current >= 60.0) && s.potential >= 40.0;
        out[i] = if pct < 0.25 {
            if floor_ok {
                if pct < 0.02 {
                    Grade::Pepite
                } else {
                    Grade::APlus
                }
            } else {
                // Best value but fails the quality floor — not A+ material.
                Grade::A
            }
        } else if pct < 0.50 {
            Grade::A
        } else if pct < 0.75 {
            Grade::B
        } else {
            Grade::C
        };
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rank_one_on_all_keywords_scores_100() {
        let s = thematic_score(&[1, 1, 1, 1], 4);
        assert_eq!(s.current, 100.0);
        assert_eq!(s.potential, 100.0);
    }

    #[test]
    fn broad_coverage_lifts_current_over_narrow() {
        // Broad: ranks (rank 5) on 4 of 4 keywords.
        let broad = thematic_score(&[5, 5, 5, 5], 4);
        // Narrow: ranks #1 on 1 of 4 keywords.
        let narrow = thematic_score(&[1], 4);
        // Narrow has the perfect position where present → higher potential…
        assert!(narrow.potential > broad.potential);
        // …but broad weighs more across the whole thematic → higher current.
        assert!(broad.current > narrow.current);
    }

    #[test]
    fn missing_keywords_drag_current_but_not_potential() {
        // Ranks #1 on 2 keywords out of a 10-keyword thematic.
        let s = thematic_score(&[1, 1], 10);
        assert_eq!(s.potential, 100.0); // perfect where present
        assert_eq!(s.current, 20.0); // 2.0 / 10 * 100
    }

    #[test]
    fn empty_scores_zero() {
        let s = thematic_score(&[], 10);
        assert_eq!(s.current, 0.0);
        assert_eq!(s.potential, 0.0);
        let z = thematic_score(&[1, 2, 3], 0);
        assert_eq!(z.current, 0.0);
    }

    fn inp(current: f64, potential: f64, price: Option<i64>, pages: u32) -> SpotGradeInput {
        SpotGradeInput {
            current,
            potential,
            price_cents: price,
            distinct_pages: pages,
        }
    }

    #[test]
    fn unpriced_or_unperforming_is_unrated() {
        let g = grade_set(&[
            inp(50.0, 50.0, None, 3),      // no price
            inp(0.0, 0.0, Some(10000), 3), // no performance
        ]);
        assert_eq!(g[0], Grade::Unrated);
        assert_eq!(g[1], Grade::Unrated);
    }

    #[test]
    fn best_value_with_floor_is_pepite_worst_is_c() {
        // Cheapest-per-performance first. 100 spots so quartiles are clean.
        let mut spots = Vec::new();
        for k in 0..100 {
            // price rises with k, performance fixed → ratio rises with k.
            spots.push(inp(70.0, 50.0, Some(1000 + k as i64 * 1000), 5));
        }
        let g = grade_set(&spots);
        assert_eq!(g[0], Grade::Pepite); // top 2%, floor cleared
        assert_eq!(g[99], Grade::C); // worst quartile
        assert!(g.iter().any(|&x| x == Grade::APlus));
        assert!(g.iter().any(|&x| x == Grade::A));
        assert!(g.iter().any(|&x| x == Grade::B));
    }

    #[test]
    fn top_value_failing_floor_drops_to_a() {
        // Two spots; the cheaper one fails the floor (1 page, low current & potential).
        let g = grade_set(&[
            inp(30.0, 20.0, Some(1000), 1), // best ratio but floor fails (potential<40)
            inp(80.0, 90.0, Some(9000), 5), // worse ratio, clears floor
        ]);
        assert_eq!(g[0], Grade::A); // demoted from A+/Pepite
    }
}
