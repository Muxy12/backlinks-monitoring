//! Keyword-candidate extraction from a site's own `<h1>` headings (Spot Finder,
//! Lot A). Pure: a heading string in, a cleaned candidate out — no I/O.
//!
//! These candidates seed keyword-based prospecting, but the user reviews, edits
//! and prunes them before any paid SERP lookup, so the filtering here is
//! deliberately light: drop only the obviously-unusable (empty, far too long,
//! no letters), and leave judgement calls to the review screen. We do NOT try to
//! split brand suffixes or guess "real" keywords — that risks discarding good
//! ones, and the human is the gate.

/// A cleaned keyword candidate: a `display` form (shown and editable) and a
/// `normalized` key used to dedup within a project.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeywordCandidate {
    pub display: String,
    pub normalized: String,
}

/// Characters stripped from the ends of a heading: wrapping quotes and trailing
/// separators sites use before a brand ("Backlink monitoring |", "Guide –").
const EDGE_TRIM: &[char] = &['"', '\'', '«', '»', '|', ':', '–', '—', '-', '·', '•'];

/// Clean one raw heading into a candidate, or `None` if it is unusable.
pub fn clean_keyword(raw: &str) -> Option<KeywordCandidate> {
    // Collapse all whitespace (incl. newlines inside a heading) to single spaces.
    let collapsed = raw.split_whitespace().collect::<Vec<_>>().join(" ");
    // Trim whitespace and edge punctuation from both ends.
    let display = collapsed
        .trim_matches(|c: char| c.is_whitespace() || EDGE_TRIM.contains(&c))
        .to_string();

    let len = display.chars().count();
    if !(2..=80).contains(&len) {
        return None;
    }
    if !display.chars().any(|c| c.is_alphabetic()) {
        return None;
    }
    let normalized = display.to_lowercase();
    Some(KeywordCandidate {
        display,
        normalized,
    })
}

/// Clean a batch of headings into candidates, deduped by their normalized key
/// (first occurrence wins, preserving document order).
pub fn candidates_from_h1s(h1s: &[String]) -> Vec<KeywordCandidate> {
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    for h in h1s {
        if let Some(c) = clean_keyword(h)
            && seen.insert(c.normalized.clone())
        {
            out.push(c);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collapses_whitespace_and_trims() {
        let c = clean_keyword("  Backlink   monitoring\n  tool  ").unwrap();
        assert_eq!(c.display, "Backlink monitoring tool");
        assert_eq!(c.normalized, "backlink monitoring tool");
    }

    #[test]
    fn strips_wrapping_quotes_and_trailing_separators() {
        assert_eq!(
            clean_keyword("\"SEO acquisition\"").unwrap().display,
            "SEO acquisition"
        );
        assert_eq!(
            clean_keyword("Backlink monitoring |").unwrap().display,
            "Backlink monitoring"
        );
        assert_eq!(clean_keyword("Guide –").unwrap().display, "Guide");
    }

    #[test]
    fn rejects_empty_too_short_and_too_long() {
        assert!(clean_keyword("").is_none());
        assert!(clean_keyword("   ").is_none());
        assert!(clean_keyword("a").is_none());
        assert!(clean_keyword(&"x".repeat(200)).is_none());
    }

    #[test]
    fn rejects_headings_without_letters() {
        assert!(clean_keyword("12345").is_none());
        assert!(clean_keyword("— · —").is_none());
    }

    #[test]
    fn keeps_accents_and_non_ascii() {
        let c = clean_keyword("Référencement naturel").unwrap();
        assert_eq!(c.display, "Référencement naturel");
        assert_eq!(c.normalized, "référencement naturel");
    }

    #[test]
    fn dedups_by_normalized_key_preserving_order() {
        let h1s = vec![
            "Backlink Monitoring".to_string(),
            "backlink monitoring".to_string(), // dup (case)
            "Link building".to_string(),
            "".to_string(),               // dropped
            "Link Building ".to_string(), // dup (case + trailing space)
        ];
        let out = candidates_from_h1s(&h1s);
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].display, "Backlink Monitoring");
        assert_eq!(out[1].display, "Link building");
    }
}
