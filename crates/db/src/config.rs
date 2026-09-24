//! Database configuration and the Neon connection decisions (ADR-0005).
//!
//! The interesting piece is [`resolve_statement_cache_capacity`], which encodes
//! open item **R1** from the research: SQLx's per-connection prepared-statement
//! cache uses named statements that can collide against a transaction-mode
//! pooler (`prepared statement "sqlx_s_N" already exists`). PgBouncer ≥ 1.22
//! *should* make protocol-level prepared statements safe, but the field
//! evidence is mixed, so we default to the setting that cannot break — cache
//! **off** whenever the DSN points at a pooled (`-pooler`) Neon host — and let
//! an integration test tell us whether we can safely raise it
//! (docs/01 §2.3, docs/adr/0005).

use std::time::Duration;

/// Everything needed to build a pool. Constructed from the environment, but the
/// decision logic is pure and unit-tested.
#[derive(Debug, Clone)]
pub struct DbConfig {
    /// The DSN. Pooled (`-pooler`) for the API, direct for the worker and
    /// migrations (ADR-0005).
    pub url: String,
    pub max_connections: u32,
    pub min_connections: u32,
    pub acquire_timeout: Duration,
    pub idle_timeout: Duration,
    pub max_lifetime: Duration,
    /// Per-connection prepared-statement cache size. See the module docs.
    pub statement_cache_capacity: usize,
}

/// Sensible defaults for a long-running server. Callers override per role.
impl DbConfig {
    /// Build a config for the **API** (pooled endpoint, small pool — Neon's
    /// PgBouncer absorbs the fan-out, and we must not "double pool", docs/01 §2.4).
    pub fn api(
        url: impl Into<String>,
        max_connections: u32,
        cache_override: Option<usize>,
    ) -> Self {
        let url = url.into();
        DbConfig {
            statement_cache_capacity: resolve_statement_cache_capacity(&url, cache_override),
            url,
            max_connections,
            min_connections: 0, // a scaled-to-zero instance should hold nothing
            acquire_timeout: Duration::from_secs(10),
            idle_timeout: Duration::from_secs(10 * 60),
            max_lifetime: Duration::from_secs(30 * 60),
        }
    }

    /// Build a config for the **worker** (direct endpoint, a couple of
    /// long-lived processes, warm pool to avoid reconnect churn per poll).
    pub fn worker(
        url: impl Into<String>,
        max_connections: u32,
        cache_override: Option<usize>,
    ) -> Self {
        let url = url.into();
        DbConfig {
            statement_cache_capacity: resolve_statement_cache_capacity(&url, cache_override),
            url,
            max_connections,
            min_connections: 2,
            acquire_timeout: Duration::from_secs(10),
            idle_timeout: Duration::from_secs(10 * 60),
            max_lifetime: Duration::from_secs(30 * 60),
        }
    }
}

/// Decide the SQLx statement-cache capacity for a DSN (open item R1).
///
/// - An explicit `override_` always wins (env `DB_STATEMENT_CACHE_CAPACITY`).
/// - Otherwise: **0** (off) when the host looks pooled (`-pooler`), else 100.
///
/// Off-by-default on the pooler is the setting that cannot produce
/// `prepared statement already exists`; the cost is re-parsing each query. The
/// integration test in `tests/` is what licenses raising it.
pub fn resolve_statement_cache_capacity(url: &str, override_: Option<usize>) -> usize {
    if let Some(n) = override_ {
        return n;
    }
    if is_pooled_host(url) { 0 } else { 100 }
}

/// True when the DSN's host contains `-pooler`, Neon's marker for the pooled
/// (PgBouncer) endpoint (docs/01 §2.1). Parsed from the host specifically, so a
/// `-pooler` substring elsewhere (path, password) does not trip it.
pub fn is_pooled_host(url: &str) -> bool {
    host_of_dsn(url).is_some_and(|h| h.contains("-pooler"))
}

/// Extract the host from a `postgres://[user[:pass]@]host[:port]/db?...` DSN
/// without pulling a URL crate into this hot-path decision. Returns the host
/// lowercased.
fn host_of_dsn(url: &str) -> Option<String> {
    // Strip scheme.
    let after_scheme = url.split("://").nth(1)?;
    // Drop anything from the path/query onward.
    let authority = after_scheme
        .split(['/', '?'])
        .next()
        .unwrap_or(after_scheme);
    // Drop userinfo (everything up to and including the last '@').
    let host_port = match authority.rsplit_once('@') {
        Some((_, hp)) => hp,
        None => authority,
    };
    // Drop the port. Handle bracketed IPv6 literals.
    let host = if let Some(rest) = host_port.strip_prefix('[') {
        rest.split(']').next().unwrap_or(rest)
    } else {
        host_port.split(':').next().unwrap_or(host_port)
    };
    if host.is_empty() {
        None
    } else {
        Some(host.to_ascii_lowercase())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const POOLED: &str = "postgres://u:p@ep-cool-name-123456-pooler.eu-central-1.aws.neon.tech/backlio?sslmode=require";
    const DIRECT: &str =
        "postgres://u:p@ep-cool-name-123456.eu-central-1.aws.neon.tech/backlio?sslmode=require";

    #[test]
    fn detects_pooled_vs_direct_host() {
        assert!(is_pooled_host(POOLED));
        assert!(!is_pooled_host(DIRECT));
        assert!(!is_pooled_host(
            "postgres://backlio:backlio@localhost:5432/backlio"
        ));
    }

    #[test]
    fn pooler_substring_in_password_does_not_trip_detection() {
        // The password contains "-pooler" but the host does not.
        let dsn = "postgres://user:x-pooler-y@db.example.com:5432/backlio";
        assert!(
            !is_pooled_host(dsn),
            "must parse the host, not substring the whole DSN"
        );
    }

    #[test]
    fn cache_defaults_off_for_pooled_on_for_direct() {
        // R1: the safe default. Pooled -> 0, direct -> 100.
        assert_eq!(resolve_statement_cache_capacity(POOLED, None), 0);
        assert_eq!(resolve_statement_cache_capacity(DIRECT, None), 100);
    }

    #[test]
    fn explicit_override_always_wins() {
        assert_eq!(resolve_statement_cache_capacity(POOLED, Some(250)), 250);
        assert_eq!(resolve_statement_cache_capacity(DIRECT, Some(0)), 0);
    }

    #[test]
    fn host_parsing_handles_ipv6_and_no_userinfo() {
        assert_eq!(
            host_of_dsn("postgres://[2001:db8::1]:5432/db").as_deref(),
            Some("2001:db8::1")
        );
        assert_eq!(
            host_of_dsn("postgres://host.example.com/db").as_deref(),
            Some("host.example.com")
        );
        assert_eq!(
            host_of_dsn("postgres://u:p@H.EXAMPLE.com/db").as_deref(),
            Some("h.example.com")
        );
    }

    #[test]
    fn api_pool_is_small_and_holds_nothing_idle() {
        let cfg = DbConfig::api(POOLED, 8, None);
        assert_eq!(cfg.max_connections, 8);
        assert_eq!(
            cfg.min_connections, 0,
            "API must not hold idle connections (double-pooling)"
        );
        assert_eq!(
            cfg.statement_cache_capacity, 0,
            "pooled API -> cache off (R1)"
        );
    }

    #[test]
    fn worker_keeps_a_warm_direct_pool() {
        let cfg = DbConfig::worker(DIRECT, 16, None);
        assert_eq!(cfg.max_connections, 16);
        assert_eq!(cfg.min_connections, 2);
        assert_eq!(
            cfg.statement_cache_capacity, 100,
            "direct worker -> cache on"
        );
    }
}
