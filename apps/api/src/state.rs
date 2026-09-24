//! Shared application state and configuration.

use auth::Argon2Params;
use db::sqlx::PgPool;

/// Runtime configuration the handlers need. Cheap to clone.
#[derive(Debug, Clone)]
pub struct ApiConfig {
    pub argon: Argon2Params,
    pub session_ttl_hours: i64,
    /// Set `Secure` on cookies (true in production; false for local http tests).
    pub secure_cookies: bool,
}

impl Default for ApiConfig {
    fn default() -> Self {
        ApiConfig {
            argon: Argon2Params::default(),
            session_ttl_hours: 720,
            secure_cookies: true,
        }
    }
}

/// State shared across handlers. `PgPool` is an `Arc` inside, so cloning is
/// cheap and this is passed by value into the router.
#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub cfg: ApiConfig,
}

impl AppState {
    pub fn new(pool: PgPool, cfg: ApiConfig) -> Self {
        AppState { pool, cfg }
    }
}
