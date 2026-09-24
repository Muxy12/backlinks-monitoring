//! Backlio HTTP API (docs/09). `app()` builds the whole router so both the
//! container `server` binary and any other host share one definition (ADR-0002).

pub mod error;
pub mod routes;
pub mod session;
pub mod state;

use axum::Router;
use axum::extract::{DefaultBodyLimit, State};
use axum::routing::{get, post};

use crate::error::ApiError;
pub use crate::state::{ApiConfig, AppState};

/// The complete API router. Mounted at the root; routes carry their own
/// `/v1` prefix (docs/09 §1).
pub fn app(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/ready", get(ready))
        .route("/v1/auth/register", post(routes::auth_routes::register))
        .route("/v1/auth/login", post(routes::auth_routes::login))
        .route("/v1/auth/logout", post(routes::auth_routes::logout))
        .route("/v1/me", get(routes::me_routes::me))
        .route(
            "/v1/organizations",
            get(routes::org_routes::list).post(routes::org_routes::create),
        )
        // Global 1 MiB body cap (docs/09 §1, S7). The CSV upload route will
        // raise this locally when it lands.
        .layer(DefaultBodyLimit::max(1024 * 1024))
        .with_state(state)
}

/// Liveness — no dependencies.
async fn health() -> &'static str {
    "ok"
}

/// Readiness — confirms the database is reachable (docs/09 §2).
async fn ready(State(state): State<AppState>) -> Result<&'static str, ApiError> {
    db::health::ping(&state.pool)
        .await
        .map_err(|_| ApiError::Internal)?;
    Ok("ready")
}
