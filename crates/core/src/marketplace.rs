//! Offer comparison for price-compare. Pure: the offers found for one domain
//! across marketplaces in, the one to feature out — no I/O.
//!
//! A spot can be listed on several marketplaces at different prices and terms
//! (dofollow or not). The product's job is to show the buyer the *best* way to
//! place a link, so this is the one place "best offer" is defined — and it is an
//! explainable rule, not a black box: cheapest wins, a priced offer always beats
//! an unpriced one, and a dofollow link breaks a price tie (it is worth more).
//! Never invents a price; an unknown price ranks last, it is not treated as 0.

/// The comparable terms of one marketplace offer. The caller keeps the full
/// offer (source, url, …) and passes just what the ranking needs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Offer {
    /// Placement price in minor units; `None` = price unknown (ranks last).
    pub price_cents: Option<i64>,
    /// Whether the placement is a dofollow link; `None` = unknown.
    pub dofollow: Option<bool>,
}

// Lower tuple = better. (priced-first, then price, then dofollow-first.)
fn rank(o: &Offer) -> (u8, i64, u8) {
    let priced = u8::from(o.price_cents.is_none()); // 0 priced, 1 unpriced
    let price = o.price_cents.unwrap_or(i64::MAX);
    let follow = u8::from(o.dofollow != Some(true)); // 0 dofollow, 1 otherwise
    (priced, price, follow)
}

/// Index of the offer to feature as the headline "buy" option, or `None` when
/// there are no offers. Deterministic; the first offer wins an exact tie.
pub fn best_offer(offers: &[Offer]) -> Option<usize> {
    offers
        .iter()
        .enumerate()
        .min_by(|(_, a), (_, b)| rank(a).cmp(&rank(b)))
        .map(|(i, _)| i)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn offer(price: Option<i64>, dofollow: Option<bool>) -> Offer {
        Offer {
            price_cents: price,
            dofollow,
        }
    }

    #[test]
    fn none_when_empty() {
        assert_eq!(best_offer(&[]), None);
    }

    #[test]
    fn single_offer_is_the_best() {
        assert_eq!(best_offer(&[offer(Some(5000), None)]), Some(0));
    }

    #[test]
    fn cheaper_offer_wins() {
        let offers = [
            offer(Some(9000), Some(true)),
            offer(Some(4000), Some(false)),
        ];
        assert_eq!(best_offer(&offers), Some(1));
    }

    #[test]
    fn priced_beats_unpriced_even_if_unpriced_is_dofollow() {
        let offers = [offer(None, Some(true)), offer(Some(8000), Some(false))];
        assert_eq!(best_offer(&offers), Some(1));
    }

    #[test]
    fn dofollow_breaks_a_price_tie() {
        let offers = [
            offer(Some(5000), Some(false)),
            offer(Some(5000), Some(true)),
        ];
        assert_eq!(best_offer(&offers), Some(1));
    }

    #[test]
    fn among_unpriced_dofollow_is_preferred() {
        let offers = [offer(None, Some(false)), offer(None, Some(true))];
        assert_eq!(best_offer(&offers), Some(1));
    }

    #[test]
    fn exact_tie_keeps_the_first() {
        let offers = [offer(Some(5000), Some(true)), offer(Some(5000), Some(true))];
        assert_eq!(best_offer(&offers), Some(0));
    }

    #[test]
    fn deterministic() {
        let offers = [offer(Some(7000), None), offer(Some(3000), Some(true))];
        assert_eq!(best_offer(&offers), best_offer(&offers));
    }
}
