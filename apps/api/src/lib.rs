//! Backlio HTTP API (docs/09). `app()` builds the whole router so both the
//! container `server` binary and any other host share one definition (ADR-0002).

pub mod error;
pub mod routes;
pub mod security;
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
            "/v1/invitations/accept",
            post(routes::team_routes::accept_invitation),
        )
        .route(
            "/v1/organizations/{org_id}/members",
            get(routes::team_routes::list_members),
        )
        .route(
            "/v1/organizations/{org_id}/invitations",
            get(routes::team_routes::list_invitations).post(routes::team_routes::create_invitation),
        )
        .route(
            "/v1/organizations/{org_id}/invitations/{invitation_id}",
            axum::routing::delete(routes::team_routes::revoke_invitation),
        )
        .route(
            "/v1/organizations/{org_id}/clients",
            get(routes::resource_routes::list_clients).post(routes::resource_routes::create_client),
        )
        .route(
            "/v1/organizations/{org_id}/clients/{client_id}",
            axum::routing::delete(routes::resource_routes::delete_client),
        )
        .route(
            "/v1/organizations/{org_id}/projects",
            get(routes::resource_routes::list_projects)
                .post(routes::resource_routes::create_project),
        )
        .route(
            "/v1/organizations/{org_id}/projects/{project_id}",
            get(routes::resource_routes::get_project)
                .patch(routes::resource_routes::update_project),
        )
        .route(
            "/v1/organizations/{org_id}/campaigns",
            get(routes::ops_routes::list_campaigns).post(routes::ops_routes::create_campaign),
        )
        .route(
            "/v1/organizations/{org_id}/campaigns/{campaign_id}",
            get(routes::ops_routes::get_campaign).delete(routes::ops_routes::delete_campaign),
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
            get(routes::ops_routes::get_publisher).delete(routes::ops_routes::delete_publisher),
        )
        .route(
            "/v1/organizations/{org_id}/publishers/{publisher_id}/rollup",
            get(routes::ops_routes::publisher_rollup),
        )
        .route(
            "/v1/organizations/{org_id}/tags",
            get(routes::tags_routes::list_tags).post(routes::tags_routes::create_tag),
        )
        .route(
            "/v1/organizations/{org_id}/tags/{tag_id}",
            axum::routing::delete(routes::tags_routes::delete_tag),
        )
        .route(
            "/v1/organizations/{org_id}/backlinks",
            get(routes::resource_routes::list_backlinks)
                .post(routes::resource_routes::create_backlink),
        )
        .route(
            "/v1/organizations/{org_id}/backlinks/{backlink_id}/tags",
            get(routes::tags_routes::list_backlink_tags).post(routes::tags_routes::attach_tag),
        )
        .route(
            "/v1/organizations/{org_id}/backlinks/{backlink_id}/tags/{tag_id}",
            axum::routing::delete(routes::tags_routes::detach_tag),
        )
        .route(
            "/v1/organizations/{org_id}/backlinks/{backlink_id}",
            get(routes::resource_routes::get_backlink)
                .patch(routes::resource_routes::update_backlink),
        )
        .route(
            "/v1/organizations/{org_id}/backlinks/bulk",
            post(routes::resource_routes::bulk_create_backlinks),
        )
        .route(
            "/v1/organizations/{org_id}/backlinks/{backlink_id}/check",
            post(routes::resource_routes::check_backlink_now),
        )
        .route(
            "/v1/organizations/{org_id}/backlinks/{backlink_id}/archive",
            post(routes::resource_routes::archive_backlink),
        )
        .route(
            "/v1/organizations/{org_id}/backlinks/{backlink_id}/restore",
            post(routes::resource_routes::restore_backlink),
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
            "/v1/organizations/{org_id}/notifications",
            get(routes::resource_routes::list_notifications),
        )
        .route(
            "/v1/organizations/{org_id}/notifications/{notification_id}/read",
            post(routes::resource_routes::read_notification),
        )
        .route(
            "/v1/organizations/{org_id}/notification-preferences",
            get(routes::prefs_routes::list_preferences).put(routes::prefs_routes::set_preferences),
        )
        .route(
            "/v1/organizations/{org_id}/backlinks/export",
            get(routes::resource_routes::export_backlinks),
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
        // Machine-triggered monitoring tick (docs/17), auth by SCHEDULER_SECRET.
        .route("/internal/tick", post(routes::internal_routes::tick))
        // Global 1 MiB body cap (docs/09 §1, S7). The CSV upload route will
        // raise this locally when it lands.
        .layer(DefaultBodyLimit::max(1024 * 1024))
        // Security headers on every response (docs/11 S8).
        .layer(axum::middleware::from_fn(security::security_headers))
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
