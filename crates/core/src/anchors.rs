//! Anchor text distribution — observational, never a verdict (same spirit as
//! `impact.rs`: we describe what the profile *is*, we do not hand out a "toxic
//! score"). Pure, no I/O, no clock, so the mix shown in the UI and the numbers
//! the API returns are decided in exactly one place and exhaustively tested.
//!
//! What we can honestly say and what we cannot:
//!   - We classify each anchor into Branded / Keyword / Naked-URL / Generic /
//!     Empty. That split is computable from the anchor text alone plus the
//!     project's own brand tokens.
//!   - We do NOT claim "exact-match %". True exact-match needs the page's target
//!     keyword, which LinkPulz does not store (§27 — no opaque score). The
//!     actionable, honest signal is instead **phrase concentration**: when one
//!     keyword-style phrase dominates the profile, we surface it as an
//!     observation, with Google's own guidance — not a judgement.

/// The category an anchor falls into. Order of the enum is also the display
/// order of the distribution bar (natural → optimized → absent).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnchorKind {
    /// Mentions the brand (brand name or brand domain label).
    Branded,
    /// Descriptive / keyword-rich text — the bucket that signals optimization
    /// when it dominates. We deliberately do NOT call this "exact-match".
    Keyword,
    /// The anchor is a bare URL or domain (`example.com`, `https://…`).
    NakedUrl,
    /// Low-information filler ("click here", "read more", "website").
    Generic,
    /// No visible anchor text — typically an image link.
    Empty,
}

impl AnchorKind {
    /// Every category, in display order. The distribution always reports all of
    /// them (a zero count is information too).
    pub const ALL: [AnchorKind; 5] = [
        AnchorKind::Branded,
        AnchorKind::Keyword,
        AnchorKind::NakedUrl,
        AnchorKind::Generic,
        AnchorKind::Empty,
    ];

    /// Stable machine slug (API + CSS hooks).
    pub fn slug(self) -> &'static str {
        match self {
            AnchorKind::Branded => "branded",
            AnchorKind::Keyword => "keyword",
            AnchorKind::NakedUrl => "naked_url",
            AnchorKind::Generic => "generic",
            AnchorKind::Empty => "empty",
        }
    }

    /// Human label for the UI.
    pub fn label(self) -> &'static str {
        match self {
            AnchorKind::Branded => "Branded",
            AnchorKind::Keyword => "Keyword-rich",
            AnchorKind::NakedUrl => "Naked URL",
            AnchorKind::Generic => "Generic",
            AnchorKind::Empty => "Empty / image",
        }
    }
}

/// Generic, low-information anchors. Matched against the normalized phrase
/// (trimmed, lowercased, inner whitespace collapsed). Kept deliberately small
/// and conservative — a false "generic" is worse than a missed one.
const GENERIC: &[&str] = &[
    "click here",
    "here",
    "this",
    "this link",
    "this website",
    "this site",
    "this page",
    "website",
    "web site",
    "site",
    "link",
    "click",
    "read more",
    "learn more",
    "find out more",
    "more",
    "visit",
    "visit website",
    "visit site",
    "visit us",
    "go here",
    "check it out",
    "view",
    "view more",
    "see more",
    "see here",
    "homepage",
    "home page",
    "home",
    "source",
    "reference",
    "read",
    "continue reading",
    "details",
    "full article",
];

/// Second-level / TLD-ish labels that are never the brand on their own. Used
/// when deriving brand tokens from a domain.
const DOMAIN_STOP: &[&str] = &[
    "com", "net", "org", "io", "co", "app", "dev", "ai", "www", "uk", "us", "ca", "de", "fr", "eu",
    "info", "biz", "gov", "edu", "me", "shop", "store", "online", "site", "xyz",
];

/// The project's own brand vocabulary, used to tell "mentions us" from "keyword".
#[derive(Debug, Clone, Default)]
pub struct BrandTokens {
    tokens: Vec<String>,
}

impl BrandTokens {
    /// Derive brand tokens from the project's domain and display name. From
    /// `linkpulz.com` + `LinkPulz` we learn the token `linkpulz`; the TLD and
    /// filler words are dropped. Tokens shorter than 3 chars are ignored to
    /// avoid matching initials inside unrelated words.
    pub fn from_project(domain: &str, name: &str) -> Self {
        let mut tokens: Vec<String> = Vec::new();
        let mut push = |t: &str| {
            let t = t.trim().to_ascii_lowercase();
            if t.len() >= 3 && !DOMAIN_STOP.contains(&t.as_str()) && !tokens.contains(&t) {
                tokens.push(t);
            }
        };

        // Domain: every label except the TLD-ish ones (handles example.co.uk).
        let host = domain
            .trim()
            .to_ascii_lowercase()
            .trim_start_matches("https://")
            .trim_start_matches("http://")
            .trim_start_matches("www.")
            .split('/')
            .next()
            .unwrap_or("")
            .to_string();
        for label in host.split('.') {
            push(label);
        }
        // Name: alphanumeric words.
        for word in name.split(|c: char| !c.is_ascii_alphanumeric()) {
            push(word);
        }
        BrandTokens { tokens }
    }

    /// True if the (already lowercased) anchor mentions any brand token.
    fn matches(&self, lower: &str) -> bool {
        self.tokens.iter().any(|t| lower.contains(t.as_str()))
    }

    #[cfg(test)]
    fn from_tokens(tokens: &[&str]) -> Self {
        BrandTokens {
            tokens: tokens.iter().map(|t| t.to_string()).collect(),
        }
    }
}

/// Trim, lowercase, and collapse internal whitespace to single spaces.
fn normalize(anchor: &str) -> String {
    anchor
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_lowercase()
}

/// Does this look like a bare URL or domain anchor? Conservative: no spaces,
/// and either an explicit scheme/`www.` prefix or a `label.tld` shape.
fn looks_like_url(norm: &str) -> bool {
    if norm.is_empty() || norm.contains(' ') {
        return false;
    }
    if norm.starts_with("http://") || norm.starts_with("https://") || norm.starts_with("www.") {
        return true;
    }
    let host = norm.split('/').next().unwrap_or("");
    if !host
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-')
    {
        return false;
    }
    match host.rsplit_once('.') {
        Some((label, tld)) => {
            !label.is_empty() && tld.len() >= 2 && tld.chars().all(|c| c.is_ascii_alphabetic())
        }
        None => false,
    }
}

/// Classify one anchor. Precedence: empty → naked URL → generic → branded →
/// keyword. (URL shape wins over brand so `linkpulz.com` reads as a naked URL,
/// the natural category it belongs to, not as a brand mention.)
pub fn classify(anchor: &str, brand: &BrandTokens) -> AnchorKind {
    let norm = normalize(anchor);
    if norm.is_empty() {
        return AnchorKind::Empty;
    }
    if looks_like_url(&norm) {
        return AnchorKind::NakedUrl;
    }
    if GENERIC.contains(&norm.as_str()) {
        return AnchorKind::Generic;
    }
    if brand.matches(&norm) {
        return AnchorKind::Branded;
    }
    AnchorKind::Keyword
}

// ─── Aggregate ──────────────────────────────────────────────────────────────

/// One phrase that dominates the profile is worth flagging — but only as an
/// observation. Thresholds below.
pub const CONCENTRATION_PCT: f64 = 25.0;
/// Don't flag concentration on a tiny profile (one of three is not a pattern).
pub const CONCENTRATION_MIN_TOTAL: usize = 10;

/// A category and its share of the profile.
#[derive(Debug, Clone, PartialEq)]
pub struct CategorySlice {
    pub kind: AnchorKind,
    pub count: usize,
    pub percent: f64,
}

/// A repeated anchor phrase.
#[derive(Debug, Clone, PartialEq)]
pub struct TopAnchor {
    pub anchor: String,
    pub kind: AnchorKind,
    pub count: usize,
    pub percent: f64,
}

/// The full distribution of a set of anchors.
#[derive(Debug, Clone, PartialEq)]
pub struct Distribution {
    /// Total anchors counted (including empty ones).
    pub total: usize,
    /// All five categories, in display order, always present.
    pub categories: Vec<CategorySlice>,
    /// Most-repeated non-empty phrases, highest first.
    pub top_anchors: Vec<TopAnchor>,
    /// Distinct non-empty phrases — a plain diversity count.
    pub distinct: usize,
    /// If one keyword-style phrase dominates, the phrase + its share. This is an
    /// observation ("X% are the single phrase '…'"), never a verdict.
    pub over_concentration: Option<TopAnchor>,
}

fn pct(count: usize, total: usize) -> f64 {
    if total == 0 {
        0.0
    } else {
        (count as f64) * 100.0 / (total as f64)
    }
}

/// Aggregate a set of anchors (as stored: `None`/blank = no anchor text) into a
/// distribution. `top_n` caps the repeated-anchor list. Deterministic: ties
/// break on the phrase, so the same input always yields the same output.
pub fn distribution(anchors: &[Option<String>], brand: &BrandTokens, top_n: usize) -> Distribution {
    let total = anchors.len();

    let mut counts = [0usize; 5];
    // phrase key (normalized) → (display, kind, count), insertion order preserved
    // for stable first-seen display form.
    let mut phrases: Vec<(String, String, AnchorKind, usize)> = Vec::new();

    for a in anchors {
        let raw = a.as_deref().unwrap_or("");
        let kind = classify(raw, brand);
        let idx = AnchorKind::ALL.iter().position(|k| *k == kind).unwrap();
        counts[idx] += 1;
        if kind == AnchorKind::Empty {
            continue;
        }
        let key = normalize(raw);
        let display = raw.split_whitespace().collect::<Vec<_>>().join(" ");
        match phrases.iter_mut().find(|(k, ..)| *k == key) {
            Some(entry) => entry.3 += 1,
            None => phrases.push((key, display, kind, 1)),
        }
    }

    let categories = AnchorKind::ALL
        .iter()
        .enumerate()
        .map(|(i, k)| CategorySlice {
            kind: *k,
            count: counts[i],
            percent: pct(counts[i], total),
        })
        .collect();

    // Sort phrases by count desc, then phrase asc (stable, deterministic).
    phrases.sort_by(|a, b| b.3.cmp(&a.3).then_with(|| a.0.cmp(&b.0)));

    let top_anchors: Vec<TopAnchor> = phrases
        .iter()
        .take(top_n)
        .map(|(_, display, kind, count)| TopAnchor {
            anchor: display.clone(),
            kind: *kind,
            count: *count,
            percent: pct(*count, total),
        })
        .collect();

    // Over-concentration: the top KEYWORD phrase, if it dominates a non-trivial
    // profile. Brand/naked/generic dominance is healthy, so it never triggers.
    let over_concentration = phrases
        .iter()
        .find(|(_, _, kind, _)| *kind == AnchorKind::Keyword)
        .filter(|(_, _, _, count)| {
            total >= CONCENTRATION_MIN_TOTAL && pct(*count, total) >= CONCENTRATION_PCT
        })
        .map(|(_, display, kind, count)| TopAnchor {
            anchor: display.clone(),
            kind: *kind,
            count: *count,
            percent: pct(*count, total),
        });

    Distribution {
        total,
        categories,
        top_anchors,
        distinct: phrases.len(),
        over_concentration,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn brand() -> BrandTokens {
        BrandTokens::from_tokens(&["linkpulz", "acme"])
    }

    #[test]
    fn empty_and_blank_are_empty() {
        assert_eq!(classify("", &brand()), AnchorKind::Empty);
        assert_eq!(classify("   ", &brand()), AnchorKind::Empty);
    }

    #[test]
    fn naked_urls_detected() {
        for s in [
            "https://example.com/page",
            "http://example.com",
            "www.example.com",
            "example.com",
            "example.com/blog/post",
            "sub.example.co.uk",
        ] {
            assert_eq!(classify(s, &brand()), AnchorKind::NakedUrl, "{s}");
        }
    }

    #[test]
    fn url_wins_over_brand() {
        // The brand's own domain is still a naked URL, not "branded".
        assert_eq!(classify("linkpulz.com", &brand()), AnchorKind::NakedUrl);
    }

    #[test]
    fn generic_filler_detected() {
        for s in ["click here", "Read More", "  here ", "website", "VISIT"] {
            assert_eq!(classify(s, &brand()), AnchorKind::Generic, "{s}");
        }
    }

    #[test]
    fn branded_detected() {
        assert_eq!(classify("LinkPulz", &brand()), AnchorKind::Branded);
        assert_eq!(
            classify("the LinkPulz dashboard", &brand()),
            AnchorKind::Branded
        );
        assert_eq!(classify("Acme Inc", &brand()), AnchorKind::Branded);
    }

    #[test]
    fn keyword_is_the_fallback() {
        assert_eq!(
            classify("best backlink monitoring tool", &brand()),
            AnchorKind::Keyword
        );
    }

    #[test]
    fn brand_tokens_from_project_drop_tld_and_filler() {
        let b = BrandTokens::from_project("linkpulz.com", "LinkPulz");
        assert_eq!(classify("linkpulz", &b), AnchorKind::Branded);
        // TLD alone is not a brand token, so it never reads as Branded.
        assert_ne!(classify("com", &b), AnchorKind::Branded);
    }

    #[test]
    fn brand_tokens_handle_multiword_name_and_cctld() {
        let b = BrandTokens::from_project("example.co.uk", "Example Media Group");
        assert_eq!(classify("example", &b), AnchorKind::Branded);
        assert_eq!(classify("media group review", &b), AnchorKind::Branded);
        // "co" / "uk" must not become brand tokens.
        assert_eq!(classify("best uk tools", &b), AnchorKind::Keyword);
    }

    #[test]
    fn distribution_counts_all_categories_and_sums_to_total() {
        let anchors = vec![
            Some("LinkPulz".to_string()),      // branded
            Some("example.com".to_string()),   // naked url
            Some("click here".to_string()),    // generic
            Some("best seo tool".to_string()), // keyword
            None,                              // empty
            Some("".to_string()),              // empty
        ];
        let d = distribution(&anchors, &brand(), 10);
        assert_eq!(d.total, 6);
        let sum: usize = d.categories.iter().map(|c| c.count).sum();
        assert_eq!(sum, 6);
        let get = |k: AnchorKind| d.categories.iter().find(|c| c.kind == k).unwrap().count;
        assert_eq!(get(AnchorKind::Branded), 1);
        assert_eq!(get(AnchorKind::NakedUrl), 1);
        assert_eq!(get(AnchorKind::Generic), 1);
        assert_eq!(get(AnchorKind::Keyword), 1);
        assert_eq!(get(AnchorKind::Empty), 2);
    }

    #[test]
    fn top_anchors_group_case_insensitively_and_rank_by_count() {
        let anchors = vec![
            Some("Best SEO Tool".to_string()),
            Some("best seo tool".to_string()),
            Some("  best   seo   tool ".to_string()),
            Some("other phrase".to_string()),
        ];
        let d = distribution(&anchors, &brand(), 10);
        assert_eq!(d.top_anchors[0].count, 3);
        assert_eq!(d.top_anchors[0].anchor, "Best SEO Tool"); // first-seen display
        assert_eq!(d.distinct, 2);
    }

    #[test]
    fn empty_anchors_are_not_counted_as_distinct_phrases() {
        let anchors = vec![None, Some("".to_string()), Some("  ".to_string())];
        let d = distribution(&anchors, &brand(), 10);
        assert_eq!(d.distinct, 0);
        assert!(d.top_anchors.is_empty());
        assert_eq!(d.over_concentration, None);
    }

    #[test]
    fn over_concentration_flags_dominant_keyword_only() {
        // 12 anchors, 6 the same keyword phrase (50%).
        let mut anchors: Vec<Option<String>> = Vec::new();
        for _ in 0..6 {
            anchors.push(Some("cheap car insurance".to_string()));
        }
        for i in 0..6 {
            anchors.push(Some(format!("filler phrase {i}")));
        }
        let d = distribution(&anchors, &brand(), 10);
        let oc = d.over_concentration.expect("should flag");
        assert_eq!(oc.anchor, "cheap car insurance");
        assert_eq!(oc.count, 6);
        assert!((oc.percent - 50.0).abs() < 1e-9);
    }

    #[test]
    fn branded_dominance_is_not_flagged() {
        // The brand as the most common anchor is healthy, never flagged.
        let mut anchors: Vec<Option<String>> = Vec::new();
        for _ in 0..8 {
            anchors.push(Some("LinkPulz".to_string()));
        }
        for i in 0..4 {
            anchors.push(Some(format!("misc {i}")));
        }
        let d = distribution(&anchors, &brand(), 10);
        assert_eq!(d.over_concentration, None);
    }

    #[test]
    fn small_profile_is_not_flagged() {
        let anchors = vec![
            Some("cheap car insurance".to_string()),
            Some("cheap car insurance".to_string()),
            Some("other".to_string()),
        ];
        let d = distribution(&anchors, &brand(), 10);
        assert_eq!(d.over_concentration, None); // below CONCENTRATION_MIN_TOTAL
    }

    #[test]
    fn deterministic() {
        let anchors = vec![
            Some("a phrase".to_string()),
            Some("b phrase".to_string()),
            Some("a phrase".to_string()),
        ];
        let d1 = distribution(&anchors, &brand(), 10);
        let d2 = distribution(&anchors, &brand(), 10);
        assert_eq!(d1, d2);
    }
}
