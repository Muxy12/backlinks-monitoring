//! URL normalization (docs/06 §3.1). Pure logic, shared by the crawler's link
//! matching and by `backlinks.*_normalized` at write time (docs/03 §3.4), so a
//! link discovered on a page and a link stored in the database compare on
//! identical terms. Getting this consistent is what prevents false
//! `TARGET_CHANGED` events across the harmless `http`↔`https` / `www` variations.

use url::Url;

/// Query parameters stripped before comparison — pure tracking noise that does
/// not identify the destination.
const TRACKING_PREFIXES: &[&str] = &["utm_"];
const TRACKING_EXACT: &[&str] = &[
    "gclid", "fbclid", "mc_eid", "mc_cid", "ref", "ref_src", "igshid", "yclid", "_hsenc", "_hsmi",
];

/// How much to fold when comparing two URLs.
#[derive(Debug, Clone, Copy)]
pub struct NormalizeOptions {
    /// Treat `http` and `https` as equivalent (default true). A link that moved
    /// http→https is not a changed target (docs/06 §3.1).
    pub scheme_insensitive: bool,
    /// Treat `www.host` and `host` as equivalent (default true).
    pub strip_www: bool,
}

impl Default for NormalizeOptions {
    fn default() -> Self {
        NormalizeOptions {
            scheme_insensitive: true,
            strip_www: true,
        }
    }
}

/// Normalize a URL string to its canonical comparison form. `base` resolves a
/// relative href — it should be the page's `<base href>` or, failing that, the
/// **final** source URL after redirects (docs/06 §3.1). Returns `None` if the
/// input cannot be parsed even against the base.
pub fn normalize(input: &str, base: Option<&str>, opts: NormalizeOptions) -> Option<String> {
    let parsed = match Url::parse(input) {
        Ok(u) => u,
        Err(url::ParseError::RelativeUrlWithoutBase) => {
            let base_url = Url::parse(base?).ok()?;
            base_url.join(input).ok()?
        }
        Err(_) => return None,
    };
    Some(canonicalize(&parsed, opts))
}

fn canonicalize(u: &Url, opts: NormalizeOptions) -> String {
    // Scheme.
    let scheme = if opts.scheme_insensitive {
        "https"
    } else {
        u.scheme()
    };

    // Host: lowercase (url already lowercases), optional www strip.
    let mut host = u.host_str().unwrap_or("").to_ascii_lowercase();
    if opts.strip_www
        && let Some(rest) = host.strip_prefix("www.")
    {
        host = rest.to_string();
    }

    // Port: drop when it is the scheme default.
    let port = match (u.port(), u.scheme()) {
        (Some(80), "http") | (Some(443), "https") => String::new(),
        (Some(p), _) => format!(":{p}"),
        (None, _) => String::new(),
    };

    // Path: empty becomes "/".
    let path = if u.path().is_empty() { "/" } else { u.path() };

    // Query: strip tracking params, sort the rest for order-independence.
    let mut pairs: Vec<(String, String)> = u
        .query_pairs()
        .filter(|(k, _)| !is_tracking(k))
        .map(|(k, v)| (k.into_owned(), v.into_owned()))
        .collect();
    pairs.sort();
    let query = if pairs.is_empty() {
        String::new()
    } else {
        let mut q = String::from("?");
        for (i, (k, v)) in pairs.iter().enumerate() {
            if i > 0 {
                q.push('&');
            }
            q.push_str(k);
            if !v.is_empty() {
                q.push('=');
                q.push_str(v);
            }
        }
        q
    };

    // Fragment is always dropped.
    format!("{scheme}://{host}{port}{path}{query}")
}

fn is_tracking(key: &str) -> bool {
    let k = key.to_ascii_lowercase();
    TRACKING_EXACT.contains(&k.as_str()) || TRACKING_PREFIXES.iter().any(|p| k.starts_with(p))
}

/// The registrable-domain-agnostic host of a URL, lowercased, `www` stripped.
/// (The public-suffix-aware registrable domain is computed in the crawler,
/// which owns the PSL; this is the plain host for display and grouping.)
pub fn host_of(input: &str) -> Option<String> {
    let u = Url::parse(input).ok()?;
    let mut h = u.host_str()?.to_ascii_lowercase();
    if let Some(rest) = h.strip_prefix("www.") {
        h = rest.to_string();
    }
    Some(h)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn norm(s: &str) -> Option<String> {
        normalize(s, None, NormalizeOptions::default())
    }

    #[test]
    fn http_and_https_fold_together_by_default() {
        assert_eq!(norm("http://example.com/x"), norm("https://example.com/x"));
    }

    #[test]
    fn www_is_stripped_by_default() {
        assert_eq!(
            norm("https://www.example.com/x"),
            norm("https://example.com/x")
        );
    }

    #[test]
    fn default_ports_and_empty_path_normalize() {
        assert_eq!(
            norm("https://example.com:443"),
            norm("https://example.com/")
        );
        assert_eq!(norm("http://example.com:80/"), norm("https://example.com/"));
        assert_eq!(
            norm("https://example.com"),
            Some("https://example.com/".into())
        );
    }

    #[test]
    fn fragment_is_dropped() {
        assert_eq!(
            norm("https://example.com/x#section"),
            norm("https://example.com/x")
        );
    }

    #[test]
    fn tracking_params_are_stripped_others_kept_and_sorted() {
        assert_eq!(
            norm("https://example.com/x?utm_source=nl&b=2&a=1&gclid=xyz"),
            Some("https://example.com/x?a=1&b=2".into())
        );
    }

    #[test]
    fn non_default_port_is_kept() {
        assert_eq!(
            normalize(
                "https://example.com:8443/x",
                None,
                NormalizeOptions::default()
            ),
            Some("https://example.com:8443/x".into())
        );
    }

    #[test]
    fn relative_href_resolves_against_base() {
        assert_eq!(
            normalize(
                "/blog/post",
                Some("https://example.com/base/"),
                NormalizeOptions::default()
            ),
            Some("https://example.com/blog/post".into())
        );
        // Protocol-relative href.
        assert_eq!(
            normalize(
                "//cdn.example.com/a",
                Some("https://example.com/"),
                NormalizeOptions::default()
            ),
            norm("https://cdn.example.com/a")
        );
    }

    #[test]
    fn relative_without_base_is_none() {
        assert_eq!(normalize("/x", None, NormalizeOptions::default()), None);
    }

    #[test]
    fn idn_host_is_punycoded_by_url_crate() {
        // The `url` crate encodes IDN hosts to punycode; both spellings fold.
        let a = norm("https://münchen.example/x");
        let b = norm("https://xn--mnchen-3ya.example/x");
        assert_eq!(a, b);
        assert!(a.unwrap().contains("xn--"));
    }

    #[test]
    fn scheme_sensitive_mode_keeps_them_distinct() {
        let opts = NormalizeOptions {
            scheme_insensitive: false,
            strip_www: true,
        };
        assert_ne!(
            normalize("http://example.com/x", None, opts),
            normalize("https://example.com/x", None, opts)
        );
    }

    #[test]
    fn host_of_strips_www_and_lowercases() {
        assert_eq!(
            host_of("https://WWW.Example.com/x"),
            Some("example.com".into())
        );
        assert_eq!(
            host_of("https://blog.example.co.uk/"),
            Some("blog.example.co.uk".into())
        );
    }
}
