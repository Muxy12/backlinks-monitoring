//! Link matching: does this parsed page contain the target link, and in what
//! form? (docs/06 §3). Turns a [`ParsedPage`] plus a target URL into the
//! `Present`/`Absent` [`Observation`] the state machine consumes (docs/08 §1).

use crate::parse::ParsedPage;
use backlio_core::{Detected, NormalizeOptions, Observation, RelSet, normalize};

/// A page link that resolved to the target URL.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkMatch {
    /// The normalized href (equals the normalized target).
    pub href_normalized: String,
    pub anchor_text: Option<String>,
    pub rel: RelSet,
    /// Already accounts for page-level nofollow (docs/06 §3.3).
    pub is_dofollow: bool,
    pub position: usize,
}

/// Find every link on `page` that resolves to `target_url`. Relative hrefs
/// resolve against the page's `<base href>` if present, else `source_final_url`
/// — the URL after redirects, not the original (docs/06 §3.1).
pub fn find_matches(
    page: &ParsedPage,
    target_url: &str,
    source_final_url: &str,
    opts: NormalizeOptions,
) -> Vec<LinkMatch> {
    let Some(target_norm) = normalize(target_url, None, opts) else {
        return Vec::new();
    };
    let base = page.base_href.as_deref().unwrap_or(source_final_url);

    let mut matches = Vec::new();
    for link in &page.links {
        if let Some(href_norm) = normalize(&link.href_raw, Some(base), opts)
            && href_norm == target_norm
        {
            matches.push(LinkMatch {
                href_normalized: href_norm,
                anchor_text: link.anchor_text.clone(),
                rel: link.rel.clone(),
                is_dofollow: link.rel.is_dofollow(page.robots_nofollow),
                position: link.position,
            });
        }
    }
    matches
}

/// Pick the "best" match to represent the backlink's current state when a page
/// links to the target more than once (docs/06 §3.4): prefer a dofollow link,
/// then earliest document order. All matches are still available to the caller
/// for the check payload, so history is not lossy.
pub fn best_match(matches: &[LinkMatch]) -> Option<&LinkMatch> {
    matches.iter().min_by_key(|m| (!m.is_dofollow, m.position))
}

/// Evaluate a successfully-parsed page against a target into an [`Observation`].
/// This produces only `Present`/`Absent` — the "we fetched and parsed it" case.
/// The error observations (`SourceUnreachable`, `SourceBlocked`, …) come from
/// the fetch layer, which owns the failures parsing never sees (docs/08 §1).
pub fn evaluate(
    page: &ParsedPage,
    target_url: &str,
    source_final_url: &str,
    opts: NormalizeOptions,
) -> Observation {
    let matches = find_matches(page, target_url, source_final_url, opts);
    match best_match(&matches) {
        None => Observation::Absent,
        Some(m) => Observation::Present(Detected {
            anchor_text: m.anchor_text.clone(),
            target_url: Some(m.href_normalized.clone()),
            rel: m.rel.clone(),
            is_dofollow: m.is_dofollow,
            canonical: page.canonical.clone(),
            noindex: page.robots_noindex,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::parse_page;

    const SRC: &str = "https://blog.example.com/article";
    fn opts() -> NormalizeOptions {
        NormalizeOptions::default()
    }

    #[test]
    fn finds_an_absolute_target_link() {
        let page = parse_page(r#"<a href="https://acme.com/product">buy</a>"#);
        let m = find_matches(&page, "https://acme.com/product", SRC, opts());
        assert_eq!(m.len(), 1);
        assert!(m[0].is_dofollow);
    }

    #[test]
    fn absent_when_target_not_present() {
        let page = parse_page(r#"<a href="https://other.com/x">x</a>"#);
        assert_eq!(
            evaluate(&page, "https://acme.com/product", SRC, opts()),
            Observation::Absent
        );
    }

    #[test]
    fn relative_href_resolves_against_base_href() {
        let page = parse_page(
            r#"<head><base href="https://acme.com/shop/"></head><body><a href="product">buy</a></body>"#,
        );
        let m = find_matches(&page, "https://acme.com/shop/product", SRC, opts());
        assert_eq!(
            m.len(),
            1,
            "relative href should resolve against <base href>"
        );
    }

    #[test]
    fn relative_href_resolves_against_source_url_when_no_base() {
        let page = parse_page(r#"<a href="/product">buy</a>"#);
        // No base -> resolve against the source page URL's origin.
        let m = find_matches(&page, "https://blog.example.com/product", SRC, opts());
        assert_eq!(m.len(), 1);
    }

    #[test]
    fn http_https_and_www_variation_still_matches() {
        // Target recorded as http+www; page links to https, no www.
        let page = parse_page(r#"<a href="https://acme.com/p">buy</a>"#);
        let m = find_matches(&page, "http://www.acme.com/p", SRC, opts());
        assert_eq!(
            m.len(),
            1,
            "harmless scheme/www variation must not miss the link"
        );
    }

    #[test]
    fn nofollow_link_is_detected_but_not_dofollow() {
        let page = parse_page(r#"<a href="https://acme.com/p" rel="nofollow">buy</a>"#);
        let obs = evaluate(&page, "https://acme.com/p", SRC, opts());
        match obs {
            Observation::Present(d) => assert!(!d.is_dofollow),
            _ => panic!("expected Present"),
        }
    }

    #[test]
    fn page_level_nofollow_makes_the_match_nofollow() {
        let page = parse_page(
            r#"<head><meta name="robots" content="nofollow"></head><body><a href="https://acme.com/p">buy</a></body>"#,
        );
        let obs = evaluate(&page, "https://acme.com/p", SRC, opts());
        match obs {
            Observation::Present(d) => {
                assert!(!d.is_dofollow, "page nofollow overrides a bare link")
            }
            _ => panic!("expected Present"),
        }
    }

    #[test]
    fn multiple_matches_prefers_dofollow_then_order() {
        let page = parse_page(
            r#"<a href="https://acme.com/p" rel="nofollow">first</a>
               <a href="https://acme.com/p">second</a>"#,
        );
        let matches = find_matches(&page, "https://acme.com/p", SRC, opts());
        assert_eq!(matches.len(), 2);
        let best = best_match(&matches).unwrap();
        assert!(best.is_dofollow, "dofollow beats the earlier nofollow");
        assert_eq!(best.anchor_text.as_deref(), Some("second"));
    }

    #[test]
    fn multiple_dofollow_matches_prefers_earliest() {
        let page = parse_page(
            r#"<a href="https://acme.com/p">early</a><a href="https://acme.com/p">late</a>"#,
        );
        let matches = find_matches(&page, "https://acme.com/p", SRC, opts());
        assert_eq!(
            best_match(&matches).unwrap().anchor_text.as_deref(),
            Some("early")
        );
    }

    #[test]
    fn evaluate_carries_canonical_and_noindex() {
        let page = parse_page(
            r#"<head><link rel="canonical" href="https://acme.com/canon"><meta name="robots" content="noindex"></head>
               <body><a href="https://acme.com/p">buy</a></body>"#,
        );
        match evaluate(&page, "https://acme.com/p", SRC, opts()) {
            Observation::Present(d) => {
                assert_eq!(d.canonical.as_deref(), Some("https://acme.com/canon"));
                assert!(d.noindex);
            }
            _ => panic!("expected Present"),
        }
    }

    #[test]
    fn protocol_relative_href_matches() {
        let page = parse_page(r#"<a href="//acme.com/p">buy</a>"#);
        let m = find_matches(&page, "https://acme.com/p", SRC, opts());
        assert_eq!(
            m.len(),
            1,
            "protocol-relative href should resolve via the source scheme"
        );
    }
}
