//! Post-Link Impact — the deterministic math (BETA).
//!
//! Observation, not attribution: these functions turn before/after aggregates
//! into change percentages and signal states. They NEVER claim causation — the
//! API wraps the output in "after acquisition…" language. Pure, no I/O, so every
//! edge (zero baseline, missing data, improvement direction) is unit-tested, and
//! the formulas live in ONE place (not scattered across SQL/UI).

/// A layer's observed state. Not everything is green/red.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignalState {
    Positive,
    Neutral,
    Negative,
    InsufficientData,
    Unavailable,
    Pending,
}

impl SignalState {
    pub fn as_str(self) -> &'static str {
        match self {
            SignalState::Positive => "positive",
            SignalState::Neutral => "neutral",
            SignalState::Negative => "negative",
            SignalState::InsufficientData => "insufficient_data",
            SignalState::Unavailable => "unavailable",
            SignalState::Pending => "pending",
        }
    }
}

/// How much to trust the observation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Confidence {
    Low,
    Medium,
    High,
}

impl Confidence {
    pub fn as_str(self) -> &'static str {
        match self {
            Confidence::Low => "low",
            Confidence::Medium => "medium",
            Confidence::High => "high",
        }
    }
}

/// Default thresholds (percentage points for rates, positions for ranking).
pub const RATE_THRESHOLD_PCT: f64 = 10.0;
pub const POSITION_THRESHOLD: f64 = 1.0;

/// Per-day rate of a total over a window (`None`/0 window → 0).
pub fn per_day(total: i64, window_days: i64) -> f64 {
    if window_days <= 0 {
        0.0
    } else {
        total as f64 / window_days as f64
    }
}

/// Percentage change from `pre` to `post`. `None` when the baseline is 0 — the
/// caller distinguishes "new activity" (post > 0) from "no change"; we never
/// return +∞%.
pub fn change_percent(pre: f64, post: f64) -> Option<f64> {
    if pre <= 0.0 {
        None
    } else {
        Some(((post - pre) / pre) * 100.0)
    }
}

/// Signal for an "up is good" metric (impressions, clicks), comparing per-day
/// rates. `change` is `change_percent(pre_rate, post_rate)`.
pub fn up_signal(
    change: Option<f64>,
    pre_total: i64,
    post_total: i64,
    threshold: f64,
) -> SignalState {
    if pre_total == 0 && post_total == 0 {
        return SignalState::InsufficientData;
    }
    match change {
        Some(c) if c >= threshold => SignalState::Positive,
        Some(c) if c <= -threshold => SignalState::Negative,
        Some(_) => SignalState::Neutral,
        // Baseline was 0: brand-new activity after acquisition.
        None if post_total > 0 => SignalState::Positive,
        None => SignalState::InsufficientData,
    }
}

/// Signal for average position (LOWER is better). `improvement = pre - post`.
pub fn position_signal(pre_pos: Option<f64>, post_pos: Option<f64>, threshold: f64) -> SignalState {
    match (pre_pos, post_pos) {
        (Some(a), Some(b)) => {
            let improvement = a - b;
            if improvement >= threshold {
                SignalState::Positive
            } else if improvement <= -threshold {
                SignalState::Negative
            } else {
                SignalState::Neutral
            }
        }
        _ => SignalState::InsufficientData,
    }
}

/// Confidence from window coverage (days with data / window length) and volume.
pub fn confidence(data_days: i64, window_days: i64, impressions: i64) -> Confidence {
    let coverage = if window_days > 0 {
        data_days as f64 / window_days as f64
    } else {
        0.0
    };
    if coverage >= 0.75 && impressions >= 100 {
        Confidence::High
    } else if coverage >= 0.4 || impressions >= 20 {
        Confidence::Medium
    } else {
        Confidence::Low
    }
}

/// Combine the per-layer signals into a single headline (used for the summary):
/// positive if any layer is positive and none negative; negative if any negative
/// and none positive; mixed otherwise; else neutral/insufficient.
pub fn headline(signals: &[SignalState]) -> SignalState {
    let real: Vec<SignalState> = signals
        .iter()
        .copied()
        .filter(|s| {
            matches!(
                s,
                SignalState::Positive | SignalState::Negative | SignalState::Neutral
            )
        })
        .collect();
    if real.is_empty() {
        return SignalState::InsufficientData;
    }
    let pos = real.contains(&SignalState::Positive);
    let neg = real.contains(&SignalState::Negative);
    match (pos, neg) {
        (true, false) => SignalState::Positive,
        (false, true) => SignalState::Negative,
        (true, true) => SignalState::Neutral, // mixed → neutral headline (no false win)
        (false, false) => SignalState::Neutral,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn change_percent_handles_zero_baseline() {
        assert_eq!(change_percent(0.0, 5.0), None);
        assert_eq!(change_percent(10.0, 15.0), Some(50.0));
        assert_eq!(change_percent(10.0, 5.0), Some(-50.0));
    }

    #[test]
    fn up_signal_thresholds() {
        assert_eq!(up_signal(Some(25.0), 100, 125, 10.0), SignalState::Positive);
        assert_eq!(up_signal(Some(-30.0), 100, 70, 10.0), SignalState::Negative);
        assert_eq!(up_signal(Some(3.0), 100, 103, 10.0), SignalState::Neutral);
    }

    #[test]
    fn up_signal_new_activity_from_zero() {
        // Zero baseline + real post activity → positive ("new activity"), not +inf.
        assert_eq!(up_signal(None, 0, 40, 10.0), SignalState::Positive);
        assert_eq!(up_signal(None, 0, 0, 10.0), SignalState::InsufficientData);
    }

    #[test]
    fn position_lower_is_better() {
        // 8.0 → 4.0 is an improvement of 4 positions.
        assert_eq!(
            position_signal(Some(8.0), Some(4.0), 1.0),
            SignalState::Positive
        );
        assert_eq!(
            position_signal(Some(4.0), Some(8.0), 1.0),
            SignalState::Negative
        );
        assert_eq!(
            position_signal(Some(4.0), Some(4.3), 1.0),
            SignalState::Neutral
        );
        assert_eq!(
            position_signal(None, Some(4.0), 1.0),
            SignalState::InsufficientData
        );
    }

    #[test]
    fn confidence_levels() {
        assert_eq!(confidence(25, 28, 500), Confidence::High);
        assert_eq!(confidence(10, 28, 50), Confidence::Medium);
        assert_eq!(confidence(1, 28, 2), Confidence::Low);
    }

    #[test]
    fn headline_is_honest_about_mixed() {
        assert_eq!(
            headline(&[SignalState::Positive, SignalState::Unavailable]),
            SignalState::Positive
        );
        assert_eq!(
            headline(&[SignalState::Positive, SignalState::Negative]),
            SignalState::Neutral
        );
        assert_eq!(
            headline(&[SignalState::Unavailable, SignalState::Pending]),
            SignalState::InsufficientData
        );
    }
}
