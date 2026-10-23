//! Crawler (docs/06). Two layers:
//!
//! - **parse** (this increment): pure HTML → links / canonical / robots, and
//!   the matching that decides whether a page contains the target link. Pure
//!   Rust (`scraper`), no network, no C toolchain — fully testable offline.
//! - **fetch** (next increment): DNS resolution + per-address SSRF validation
//!   (via `backlio_core::net`), address pinning, the redirect loop that
//!   re-checks every hop, response-size cap, and per-domain rate limiting. That
//!   layer pulls `reqwest`/`hickory-resolver`.
//!
//! The SSRF *classification* already lives in `backlio_core::net`; the fetch
//! layer is the I/O around it.

pub mod config;
pub mod discovery;
pub mod fetch;
pub mod linkavista;
pub mod marketplace;
pub mod matching;
pub mod metrics;
pub mod parse;
pub mod serp;
pub mod sitescan;

pub use config::CrawlerConfig;
pub use fetch::{FetchError, FetchOk, FetchOutcome, fetch};
pub use marketplace::{MarketplaceListing, MarketplaceProvider, providers_from_env};
pub use matching::{LinkMatch, best_match, evaluate, find_matches};
pub use metrics::{DomainMetrics, MetricsProvider};
pub use parse::{ExtractedLink, ParsedPage, parse_page};
pub use serp::{Locale, SerpProvider, SerpResult};
pub use sitescan::{ScannedH1, SiteScan, scan_site};
