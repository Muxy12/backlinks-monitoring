//! Canonical keys, provider slugs and TTLs for the mutualised `provider_cache`
//! (docs/product/21 §6, 23 §4). Pure: it builds the stable `(provider,
//! cache_key)` identity and the per-kind freshness window — no I/O, no clock.
//!
//! The cache is **cross-tenant**: keys deliberately carry no organization id, so
//! a SERP / metrics / AI answer fetched for one workspace serves every workspace
//! (the margin lever, §6). Payloads are public provider data, never tenant PII.
//!
//! Keys are stored as **canonical plaintext**, not a hash: the column is `text`,
//! the inputs are short, and a readable key is far easier to debug and sweep
//! than a digest. The architecture doc's `sha(...)` was illustrative, and a
//! crypto hash would add a dependency for no real benefit at this scale.

/// Provider slugs stored in `provider_cache.provider` (match migration 0049).
pub const SERP: &str = "dfs_serp";
pub const METRICS: &str = "dfs_metrics";
pub const AI_OVERVIEW: &str = "dfs_ai_overview";

/// TTLs (seconds) per kind — doc 23 §4. SERP re-scores on a daily cadence;
/// authority/traffic move slowly; AI answers are volatile but costly.
pub const SERP_TTL_SECS: i64 = 24 * 3600;
pub const METRICS_TTL_SECS: i64 = 7 * 24 * 3600;
pub const AI_OVERVIEW_TTL_SECS: i64 = 48 * 3600;

/// `serp:{keyword}:{locale}`. The keyword is canonicalized here so trivially
/// different spellings (case, stray whitespace) share one cache entry.
pub fn serp_key(keyword: &str, locale: &str) -> String {
    format!("serp:{}:{}", canon(keyword), canon(locale))
}

/// `metrics:{domain}` — not locale-scoped (authority/traffic are global).
pub fn metrics_key(domain: &str) -> String {
    format!("metrics:{}", canon(domain))
}

/// `ai_overview:{query}:{locale}`.
pub fn ai_overview_key(query: &str, locale: &str) -> String {
    format!("ai_overview:{}:{}", canon(query), canon(locale))
}

/// Canonicalize a key component: lowercase, trim, and collapse internal runs of
/// whitespace to a single space. Deterministic and idempotent.
fn canon(s: &str) -> String {
    s.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serp_key_is_stable_across_case_and_whitespace() {
        assert_eq!(
            serp_key("Best  CRM", "en-US"),
            serp_key("best crm", "en-us")
        );
        assert_eq!(serp_key("best crm", "en"), "serp:best crm:en");
    }

    #[test]
    fn keys_differ_by_keyword_and_locale() {
        assert_ne!(serp_key("crm", "en"), serp_key("erp", "en"));
        assert_ne!(serp_key("crm", "en"), serp_key("crm", "fr"));
    }

    #[test]
    fn metrics_key_is_not_locale_scoped() {
        assert_eq!(metrics_key("Example.com"), "metrics:example.com");
        assert_eq!(metrics_key(" example.com "), metrics_key("example.com"));
    }

    #[test]
    fn ai_overview_key_shape() {
        assert_eq!(
            ai_overview_key("what is a CRM", "en"),
            "ai_overview:what is a crm:en"
        );
    }

    #[test]
    fn ttls_match_the_pipeline_doc() {
        assert_eq!(SERP_TTL_SECS, 86_400);
        assert_eq!(METRICS_TTL_SECS, 604_800);
        assert_eq!(AI_OVERVIEW_TTL_SECS, 172_800);
    }

    #[test]
    fn canon_is_idempotent() {
        let once = canon("  Mixed   Case  Text ");
        assert_eq!(once, canon(&once));
    }
}
