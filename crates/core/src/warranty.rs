//! Warranty computation (§32) — pure, no clock. A publisher may guarantee a
//! link stays live for a period; when it disappears inside that window the buyer
//! is owed a replacement or refund. This module turns the stored
//! `(warranty_type, warranty_value, acquired_date)` into a concrete expiration
//! date and classifies where "today" falls relative to it. It never reads the
//! clock — the caller passes `today`, so the same inputs always classify the
//! same way and the §32 worked example is a unit test.

use chrono::{Duration, Months, NaiveDate};

/// How a warranty's length is expressed. Mirrors the `warranty_type` enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WarrantyType {
    /// No warranty.
    None,
    /// `warranty_value` days from acquisition.
    Days,
    /// `warranty_value` calendar months from acquisition.
    Months,
    /// Never expires.
    Lifetime,
}

impl WarrantyType {
    pub fn as_str(self) -> &'static str {
        match self {
            WarrantyType::None => "none",
            WarrantyType::Days => "days",
            WarrantyType::Months => "months",
            WarrantyType::Lifetime => "lifetime",
        }
    }

    /// Parse the DB text form. Unknown values map to `None` (the safe default —
    /// a link with an unreadable warranty is treated as unwarranted, never as
    /// silently covered).
    pub fn from_db(s: &str) -> Self {
        match s {
            "days" => WarrantyType::Days,
            "months" => WarrantyType::Months,
            "lifetime" => WarrantyType::Lifetime,
            _ => WarrantyType::None,
        }
    }

    /// Whether this type needs a positive `warranty_value` to be meaningful
    /// (matches the `backlinks_warranty_value` CHECK).
    pub fn needs_value(self) -> bool {
        matches!(self, WarrantyType::Days | WarrantyType::Months)
    }
}

/// Where `today` falls relative to a link's warranty.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WarrantyStatus {
    /// No warranty was purchased.
    None,
    /// Never expires — always covered.
    Lifetime,
    /// Covered, and not near the end of the window.
    Active,
    /// Covered, but expiring within the caller's "soon" window — worth a nudge
    /// to renew or re-verify before cover lapses.
    ExpiringSoon,
    /// The window has passed; the link is no longer under warranty.
    Expired,
}

impl WarrantyStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            WarrantyStatus::None => "none",
            WarrantyStatus::Lifetime => "lifetime",
            WarrantyStatus::Active => "active",
            WarrantyStatus::ExpiringSoon => "expiring_soon",
            WarrantyStatus::Expired => "expired",
        }
    }
}

/// The date a warranty lapses, materialized on write so the DB can index it
/// (docs/03 §4.3). `None` for `none`/`lifetime`, or when a `days`/`months`
/// warranty is missing its value, or when month arithmetic overflows the
/// calendar (an absurdly large value) — in every "cannot compute" case the link
/// is treated as unwarranted rather than covered forever.
pub fn expiration(
    kind: WarrantyType,
    value: Option<i32>,
    acquired: NaiveDate,
) -> Option<NaiveDate> {
    match kind {
        WarrantyType::None | WarrantyType::Lifetime => None,
        WarrantyType::Days => {
            let v = value.filter(|v| *v > 0)?;
            acquired.checked_add_signed(Duration::days(v as i64))
        }
        WarrantyType::Months => {
            let v = value.filter(|v| *v > 0)?;
            acquired.checked_add_months(Months::new(v as u32))
        }
    }
}

/// Classify a warranty relative to `today`. `soon_window_days` is how many days
/// before expiration counts as "expiring soon" (0 disables that band). The
/// window is inclusive of both ends: on the expiration date itself the link is
/// still `ExpiringSoon` (covered), and only the following day is `Expired`.
pub fn status(
    kind: WarrantyType,
    expiration: Option<NaiveDate>,
    today: NaiveDate,
    soon_window_days: i64,
) -> WarrantyStatus {
    match kind {
        WarrantyType::None => WarrantyStatus::None,
        WarrantyType::Lifetime => WarrantyStatus::Lifetime,
        WarrantyType::Days | WarrantyType::Months => {
            // A days/months warranty with no computable expiration is not cover.
            let Some(exp) = expiration else {
                return WarrantyStatus::None;
            };
            if today > exp {
                WarrantyStatus::Expired
            } else if (exp - today).num_days() <= soon_window_days {
                WarrantyStatus::ExpiringSoon
            } else {
                WarrantyStatus::Active
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(s: &str) -> NaiveDate {
        NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap()
    }

    #[test]
    fn expiration_across_all_types() {
        let acquired = d("2026-01-15");
        // §32 worked example: a 90-day warranty bought on 2026-01-15.
        assert_eq!(
            expiration(WarrantyType::Days, Some(90), acquired),
            Some(d("2026-04-15"))
        );
        // 6 months lands on the same day-of-month, six months on.
        assert_eq!(
            expiration(WarrantyType::Months, Some(6), acquired),
            Some(d("2026-07-15"))
        );
        // none / lifetime never materialize a date.
        assert_eq!(expiration(WarrantyType::None, None, acquired), None);
        assert_eq!(expiration(WarrantyType::Lifetime, None, acquired), None);
        // days/months without a value cannot be computed -> None.
        assert_eq!(expiration(WarrantyType::Days, None, acquired), None);
        assert_eq!(expiration(WarrantyType::Months, Some(0), acquired), None);
    }

    #[test]
    fn month_arithmetic_clamps_end_of_month() {
        // Jan 31 + 1 month is Feb 28 (chrono clamps to the last valid day),
        // never a rollover into March.
        assert_eq!(
            expiration(WarrantyType::Months, Some(1), d("2026-01-31")),
            Some(d("2026-02-28"))
        );
    }

    #[test]
    fn status_bands() {
        let acquired = d("2026-01-15");
        let exp = expiration(WarrantyType::Days, Some(90), acquired); // 2026-04-15
        let soon = 14;

        // Well inside the window -> Active.
        assert_eq!(
            status(WarrantyType::Days, exp, d("2026-02-01"), soon),
            WarrantyStatus::Active
        );
        // Within 14 days of expiry -> ExpiringSoon.
        assert_eq!(
            status(WarrantyType::Days, exp, d("2026-04-05"), soon),
            WarrantyStatus::ExpiringSoon
        );
        // On the expiration date -> still covered (ExpiringSoon), not Expired.
        assert_eq!(
            status(WarrantyType::Days, exp, d("2026-04-15"), soon),
            WarrantyStatus::ExpiringSoon
        );
        // The day after -> Expired.
        assert_eq!(
            status(WarrantyType::Days, exp, d("2026-04-16"), soon),
            WarrantyStatus::Expired
        );
    }

    #[test]
    fn none_and_lifetime_status_ignore_dates() {
        let today = d("2030-01-01");
        assert_eq!(
            status(WarrantyType::None, None, today, 14),
            WarrantyStatus::None
        );
        assert_eq!(
            status(WarrantyType::Lifetime, None, today, 14),
            WarrantyStatus::Lifetime
        );
    }

    #[test]
    fn from_db_roundtrip_and_unknown_is_none() {
        for k in [
            WarrantyType::None,
            WarrantyType::Days,
            WarrantyType::Months,
            WarrantyType::Lifetime,
        ] {
            assert_eq!(WarrantyType::from_db(k.as_str()), k);
        }
        assert_eq!(WarrantyType::from_db("garbage"), WarrantyType::None);
    }
}
