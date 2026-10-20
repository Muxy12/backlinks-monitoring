//! Shared application state and configuration.

use std::sync::Arc;

use auth::Argon2Params;
use db::sqlx::PgPool;

use crate::security::CheckerLimiter;

/// Runtime configuration the handlers need. Cheap to clone.
#[derive(Debug, Clone)]
pub struct ApiConfig {
    pub argon: Argon2Params,
    pub session_ttl_hours: i64,
    /// Set `Secure` on cookies (true in production; false for local http tests).
    pub secure_cookies: bool,
    /// Stripe webhook signing secret; `None` disables the webhook endpoint.
    pub stripe_webhook_secret: Option<String>,
    /// Let the public backlink-checker fetch private/loopback addresses. MUST
    /// stay false in production (SSRF); tests flip it to reach a local mock.
    pub allow_private_fetch: bool,
    /// Link-building campaigns BETA (migration 0036). Env-gated and off by
    /// default: when false, the new campaign-workflow routes 404 and the UI
    /// hides the nav entry (surfaced via `me.features.campaigns_beta`).
    pub campaigns_beta: bool,
    /// Whether new self-serve sign-ups are accepted. Env-gated via
    /// `REGISTRATION_OPEN`; default `true`. When `false`, `POST /v1/auth/register`
    /// is refused and the public `/register` page shows a "closed" state
    /// (surfaced via `GET /v1/config`). Lets us keep the marketing site public
    /// while the product is not yet open to everyone.
    pub registration_open: bool,
    /// LinkAvista marketplace API key (per-deployment secret). `None` disables
    /// the integration: catalogue routes 404 and the UI hides the surface
    /// (surfaced via `me.features.linkavista`). The key never leaves the server.
    pub linkavista_api_key: Option<String>,
    /// Use LinkAvista sandbox mode (simulated orders, no debit). Default `true`;
    /// set `LINKAVISTA_SANDBOX=false` only once real purchasing is intended.
    /// (Lot 1 is read-only, so this is not yet consulted; it gates future orders.)
    pub linkavista_sandbox: bool,
    /// LinkAvista API base URL; `None` falls back to the public base at call time.
    pub linkavista_base_url: Option<String>,
}

impl Default for ApiConfig {
    fn default() -> Self {
        ApiConfig {
            argon: Argon2Params::default(),
            session_ttl_hours: 720,
            secure_cookies: true,
            stripe_webhook_secret: None,
            allow_private_fetch: false,
            campaigns_beta: false,
            registration_open: true,
            linkavista_api_key: None,
            linkavista_sandbox: true,
            linkavista_base_url: None,
        }
    }
}

/// State shared across handlers. `PgPool` is an `Arc` inside, so cloning is
/// cheap and this is passed by value into the router.
#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub cfg: ApiConfig,
    /// Rate limiter for the public backlink checker (docs/11 S3). Shared, so it
    /// is an `Arc` and cloning `AppState` keeps one limiter.
    pub checker: Arc<CheckerLimiter>,
}

impl AppState {
    pub fn new(pool: PgPool, cfg: ApiConfig) -> Self {
        AppState {
            pool,
            cfg,
            checker: Arc::new(CheckerLimiter::default_for_prod()),
        }
    }

    /// Build with a custom checker limiter (tests use a tiny budget).
    pub fn with_checker(pool: PgPool, cfg: ApiConfig, checker: CheckerLimiter) -> Self {
        AppState {
            pool,
            cfg,
            checker: Arc::new(checker),
        }
    }
}
