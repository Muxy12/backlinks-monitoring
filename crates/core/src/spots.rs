//! Spot aggregation (Spot Finder, Lot B). Pure: SERP hits in, ranked prospect
//! "spots" out — no I/O. A spot is a domain that ranks for one or more of the
//! user's keywords; the more of their keywords it ranks for (and the higher),
//! the stronger a prospect it is.
//!
//! We deliberately drop the user's own domain and the non-prospectable giants
//! (Wikipedia, Amazon, social networks…): you can't earn or buy a relevant link
//! from those, so they are noise in a prospecting list.

/// One organic SERP result tied to the keyword it was found for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SerpHit {
    pub keyword: String,
    pub url: String,
    /// Domain as the provider reported it (may carry `www.` / case).
    pub domain: String,
    /// Organic rank (1 = top). Lower is better.
    pub rank: i32,
}

/// An aggregated prospect: a domain ranking for ≥1 of the user's keywords.
#[derive(Debug, Clone, PartialEq)]
pub struct AggregatedSpot {
    /// Registrable-ish host (lowercased, `www.` stripped).
    pub domain: String,
    /// How many distinct keywords this domain ranks for.
    pub keyword_count: u32,
    /// Best (lowest) rank seen across those keywords.
    pub best_position: i32,
    /// Mean rank across the ranking keywords.
    pub avg_position: f64,
    /// A ranking page of this domain (the one at the best position).
    pub example_url: String,
    /// A keyword it ranks for (the one at the best position).
    pub example_keyword: String,
}

/// Registrable-ish host for grouping: lowercased, `www.` stripped, scheme/path
/// removed if a full URL slips in. Deliberately simple (no public-suffix list).
pub fn site_host(input: &str) -> String {
    let s = input.trim();
    let s = s.rsplit("://").next().unwrap_or(s);
    let s = s.split(['/', '?', '#']).next().unwrap_or(s);
    let s = s.rsplit('@').next().unwrap_or(s);
    let s = s.split(':').next().unwrap_or(s);
    // Lowercase before stripping `www.` so a capitalized "WWW." is handled too.
    let lower = s.to_ascii_lowercase();
    lower.strip_prefix("www.").unwrap_or(&lower).to_string()
}

/// Registrable domains you cannot realistically prospect for a link — platforms,
/// marketplaces and reference sites. A spot on one of these is noise.
const BLOCKED: &[&str] = &[
    "google.com",
    "youtube.com",
    "wikipedia.org",
    "amazon.com",
    "amazon.fr",
    "amazon.co.uk",
    "facebook.com",
    "instagram.com",
    "linkedin.com",
    "x.com",
    "twitter.com",
    "pinterest.com",
    "pinterest.fr",
    "reddit.com",
    "tiktok.com",
    "apple.com",
    "microsoft.com",
    "fandom.com",
    "quora.com",
    "yelp.com",
    "tripadvisor.com",
    "tripadvisor.fr",
    "booking.com",
    "ebay.com",
    "ebay.fr",
];

/// Whether a host is a blocked (non-prospectable) domain — exact match or a
/// subdomain of one (so `fr.wikipedia.org` is blocked too).
pub fn is_blocked_domain(host: &str) -> bool {
    let h = site_host(host);
    BLOCKED
        .iter()
        .any(|b| h == *b || h.ends_with(&format!(".{b}")))
}

/// True when `host` is the user's own site (same registrable host or a subdomain).
fn is_own(host: &str, own: &str) -> bool {
    !own.is_empty() && (host == own || host.ends_with(&format!(".{own}")))
}

/// Aggregate SERP hits into ranked spots. `own_domain` (any form) is excluded,
/// as are the blocked giants. Output is sorted strongest-first: more keywords,
/// then better average position, then domain for a stable order.
pub fn aggregate_spots(hits: &[SerpHit], own_domain: &str) -> Vec<AggregatedSpot> {
    use std::collections::HashMap;
    let own = site_host(own_domain);

    struct Acc {
        keywords: std::collections::HashSet<String>,
        best: i32,
        sum: i64,
        count: i64,
        example_url: String,
        example_keyword: String,
    }
    let mut map: HashMap<String, Acc> = HashMap::new();

    for h in hits {
        let host = site_host(&h.domain);
        if host.is_empty() || is_blocked_domain(&host) || is_own(&host, &own) {
            continue;
        }
        let entry = map.entry(host).or_insert_with(|| Acc {
            keywords: std::collections::HashSet::new(),
            best: i32::MAX,
            sum: 0,
            count: 0,
            example_url: h.url.clone(),
            example_keyword: h.keyword.clone(),
        });
        entry.keywords.insert(h.keyword.to_lowercase());
        entry.sum += h.rank as i64;
        entry.count += 1;
        if h.rank < entry.best {
            entry.best = h.rank;
            entry.example_url = h.url.clone();
            entry.example_keyword = h.keyword.clone();
        }
    }

    let mut out: Vec<AggregatedSpot> = map
        .into_iter()
        .map(|(domain, a)| AggregatedSpot {
            domain,
            keyword_count: a.keywords.len() as u32,
            best_position: a.best,
            avg_position: if a.count > 0 {
                a.sum as f64 / a.count as f64
            } else {
                0.0
            },
            example_url: a.example_url,
            example_keyword: a.example_keyword,
        })
        .collect();

    out.sort_by(|x, y| {
        y.keyword_count
            .cmp(&x.keyword_count)
            .then(
                x.avg_position
                    .partial_cmp(&y.avg_position)
                    .unwrap_or(std::cmp::Ordering::Equal),
            )
            .then(x.domain.cmp(&y.domain))
    });
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hit(keyword: &str, domain: &str, url: &str, rank: i32) -> SerpHit {
        SerpHit {
            keyword: keyword.into(),
            url: url.into(),
            domain: domain.into(),
            rank,
        }
    }

    #[test]
    fn site_host_normalizes() {
        assert_eq!(site_host("https://WWW.Example.com/x?y=1"), "example.com");
        assert_eq!(site_host("Blog.Example.COM"), "blog.example.com");
    }

    #[test]
    fn blocks_giants_and_subdomains() {
        assert!(is_blocked_domain("wikipedia.org"));
        assert!(is_blocked_domain("fr.wikipedia.org"));
        assert!(is_blocked_domain("www.amazon.fr"));
        assert!(!is_blocked_domain("niche-blog.com"));
    }

    #[test]
    fn aggregates_counts_best_and_average() {
        let hits = vec![
            hit("seo tools", "blog.com", "https://blog.com/a", 3),
            hit("link building", "blog.com", "https://blog.com/b", 1),
            hit("seo tools", "other.com", "https://other.com/x", 5),
        ];
        let spots = aggregate_spots(&hits, "mysite.com");
        assert_eq!(spots.len(), 2);
        // blog.com ranks for 2 keywords → first.
        assert_eq!(spots[0].domain, "blog.com");
        assert_eq!(spots[0].keyword_count, 2);
        assert_eq!(spots[0].best_position, 1);
        assert_eq!(spots[0].avg_position, 2.0);
        // Example comes from the best-ranked hit.
        assert_eq!(spots[0].example_url, "https://blog.com/b");
        assert_eq!(spots[0].example_keyword, "link building");
        assert_eq!(spots[1].domain, "other.com");
    }

    #[test]
    fn excludes_own_domain_and_blocked() {
        let hits = vec![
            hit("k", "mysite.com", "https://mysite.com/", 1),
            hit("k", "www.mysite.com", "https://www.mysite.com/x", 2),
            hit(
                "k",
                "en.wikipedia.org",
                "https://en.wikipedia.org/wiki/x",
                4,
            ),
            hit("k", "good.com", "https://good.com/", 6),
        ];
        let spots = aggregate_spots(&hits, "https://mysite.com");
        assert_eq!(spots.len(), 1);
        assert_eq!(spots[0].domain, "good.com");
    }

    #[test]
    fn same_keyword_counted_once_per_domain() {
        let hits = vec![
            hit("SEO", "blog.com", "https://blog.com/a", 2),
            hit("seo", "blog.com", "https://blog.com/b", 8), // same keyword (case)
        ];
        let spots = aggregate_spots(&hits, "x.com");
        assert_eq!(spots[0].keyword_count, 1);
        assert_eq!(spots[0].best_position, 2);
        assert_eq!(spots[0].avg_position, 5.0); // avg over both hits
    }
}
