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
        .route(
            "/v1/organizations/{org_id}/clients",
            get(routes::resource_routes::list_clients).post(routes::resource_routes::create_client),
        )
        .route(
            "/v1/organizations/{org_id}/projects",
            get(routes::resource_routes::list_projects)
                .post(routes::resource_routes::create_project),
        )
        .route(
            "/v1/organizations/{org_id}/projects/{project_id}",
            get(routes::resource_routes::get_project),
        )
        .route(
            "/v1/organizations/{org_id}/campaigns",
            get(routes::ops_routes::list_campaigns).post(routes::ops_routes::create_campaign),
        )
        .route(
            "/v1/organizations/{org_id}/campaigns/{campaign_id}",
            get(routes::ops_routes::get_campaign),
        )
        .route(
            "/v1/organizations/{org_id}/campaigns/{campaign_id}/rollup",
            get(routes::ops_routes::campaign_rollup),
        )
        .route(
            "/v1/organizations/{org_id}/publishers",
            get(routes::ops_routes::list_publishers).post(routes::ops_routes::create_publisher),
        )
        .route(
            "/v1/organizations/{org_id}/publishers/{publisher_id}",
            get(routes::ops_routes::get_publisher),
        )
        .route(
            "/v1/organizations/{org_id}/publishers/{publisher_id}/rollup",
            get(routes::ops_routes::publisher_rollup),
        )
        .route(
            "/v1/organizations/{org_id}/backlinks",
            get(routes::resource_routes::list_backlinks)
                .post(routes::resource_routes::create_backlink),
        )
        .route(
            "/v1/organizations/{org_id}/backlinks/{backlink_id}",
            get(routes::resource_routes::get_backlink),
        )
        .route(
            "/v1/organizations/{org_id}/backlinks/{backlink_id}/check",
            post(routes::resource_routes::check_backlink_now),
        )
        .route(
            "/v1/organizations/{org_id}/backlinks/{backlink_id}/events",
            get(routes::resource_routes::get_backlink_events),
        )
        .route(
            "/v1/organizations/{org_id}/dashboard",
            get(routes::resource_routes::dashboard),
        )
        .route(
            "/v1/organizations/{org_id}/warranties/expiring",
            get(routes::resource_routes::expiring_warranties),
        )
        .route(
            "/v1/organizations/{org_id}/imports",
            post(routes::resource_routes::import_csv),
        )
        .route(
            "/v1/billing/webhook",
            post(routes::billing_routes::stripe_webhook),
        )
        .route(
            "/v1/tools/backlink-check",
            post(routes::tools_routes::backlink_check),
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
