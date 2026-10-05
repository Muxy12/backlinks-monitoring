//! LinkPulz HTTP API (docs/09). `app()` builds the whole router so both the
//! container `server` binary and any other host share one definition (ADR-0002).

pub mod error;
pub mod gsc_sync;
pub mod routes;
pub mod security;
pub mod session;
pub mod state;
pub mod turnstile;

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
        .route("/v1/auth/verify", post(routes::auth_routes::verify_email))
        .route(
            "/v1/auth/resend-verification",
            post(routes::auth_routes::resend_verification),
        )
        .route(
            "/v1/auth/forgot",
            post(routes::auth_routes::forgot_password),
        )
        .route("/v1/auth/reset", post(routes::auth_routes::reset_password))
        .route("/v1/me", get(routes::me_routes::me))
        // User feedback: feature requests + bug reports (migration 0033).
        .route("/v1/feedback", post(routes::feedback_routes::submit))
        .route("/v1/feedback/mine", get(routes::feedback_routes::list_mine))
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
            axum::routing::patch(routes::resource_routes::update_client)
                .delete(routes::resource_routes::delete_client),
        )
        .route(
            "/v1/organizations/{org_id}/projects",
            get(routes::resource_routes::list_projects)
                .post(routes::resource_routes::create_project),
        )
        .route(
            "/v1/organizations/{org_id}/projects/{project_id}",
            get(routes::resource_routes::get_project)
                .patch(routes::resource_routes::update_project)
                .delete(routes::resource_routes::delete_project),
        )
        .route(
            "/v1/organizations/{org_id}/projects/{project_id}/auto-tag",
            post(routes::resource_routes::auto_tag_backlinks),
        )
        .route(
            "/v1/organizations/{org_id}/campaigns",
            get(routes::ops_routes::list_campaigns).post(routes::ops_routes::create_campaign),
        )
        .route(
            "/v1/organizations/{org_id}/campaigns/{campaign_id}",
            get(routes::ops_routes::get_campaign)
                .patch(routes::ops_routes::update_campaign)
                .delete(routes::ops_routes::delete_campaign),
        )
        .route(
            "/v1/organizations/{org_id}/campaigns/{campaign_id}/rollup",
            get(routes::ops_routes::campaign_rollup),
        )
        // Link-building campaigns BETA (migration 0036) — gated on CAMPAIGNS_BETA
        // inside each handler (404 when off).
        .route(
            "/v1/organizations/{org_id}/campaigns/{campaign_id}/summary",
            get(routes::campaigns_routes::campaign_summary),
        )
        .route(
            "/v1/organizations/{org_id}/campaigns/{campaign_id}/targets",
            get(routes::campaigns_routes::list_targets)
                .post(routes::campaigns_routes::create_target),
        )
        .route(
            "/v1/organizations/{org_id}/campaign-targets/{target_id}",
            axum::routing::delete(routes::campaigns_routes::delete_target),
        )
        .route(
            "/v1/organizations/{org_id}/campaigns/{campaign_id}/prospects",
            get(routes::campaigns_routes::list_prospects)
                .post(routes::campaigns_routes::create_prospect),
        )
        .route(
            "/v1/organizations/{org_id}/campaigns/{campaign_id}/prospects/from-gap",
            post(routes::campaigns_routes::seed_from_gap),
        )
        .route(
            "/v1/organizations/{org_id}/campaigns/{campaign_id}/prospects/bulk-stage",
            post(routes::campaigns_routes::bulk_stage),
        )
        .route(
            "/v1/organizations/{org_id}/prospects/{prospect_id}",
            axum::routing::patch(routes::campaigns_routes::update_prospect)
                .delete(routes::campaigns_routes::delete_prospect),
        )
        .route(
            "/v1/organizations/{org_id}/prospects/{prospect_id}/draft",
            post(routes::campaigns_routes::prospect_draft),
        )
        .route(
            "/v1/organizations/{org_id}/prospects/{prospect_id}/convert",
            post(routes::campaigns_routes::convert_prospect),
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
            "/v1/organizations/{org_id}/backlink-tags",
            get(routes::tags_routes::backlink_tags_bulk),
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
            "/v1/organizations/{org_id}/backlinks/bulk-monitoring",
            post(routes::resource_routes::bulk_monitoring),
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
        // Post-Link Impact (observe target-page response after acquisition).
        .route(
            "/v1/organizations/{org_id}/backlinks/{backlink_id}/impact",
            get(routes::impact_routes::get_impact),
        )
        .route(
            "/v1/organizations/{org_id}/backlinks/{backlink_id}/impact/enable",
            post(routes::impact_routes::enable_impact),
        )
        .route(
            "/v1/organizations/{org_id}/backlinks/{backlink_id}/occurrences",
            get(routes::resource_routes::list_backlink_occurrences),
        )
        .route(
            "/v1/organizations/{org_id}/backlinks/{backlink_id}/tier2",
            get(routes::resource_routes::list_tier2),
        )
        .route(
            "/v1/organizations/{org_id}/backlinks/{backlink_id}/tier2/import",
            post(routes::resource_routes::import_tier2),
        )
        .route(
            "/v1/organizations/{org_id}/dashboard",
            get(routes::resource_routes::dashboard),
        )
        // Overview / command center — one aggregation for the redesigned dashboard.
        .route(
            "/v1/organizations/{org_id}/overview",
            get(routes::overview_routes::overview),
        )
        // Google Search Console OAuth + connection (Post-Link Impact foundation).
        // The callback lands here without a session; it is authorized by the
        // signed/encrypted `state`.
        .route("/v1/gsc/oauth/callback", get(routes::gsc_routes::callback))
        .route(
            "/v1/organizations/{org_id}/projects/{project_id}/gsc/auth-url",
            get(routes::gsc_routes::auth_url),
        )
        .route(
            "/v1/organizations/{org_id}/projects/{project_id}/gsc/status",
            get(routes::gsc_routes::status),
        )
        .route(
            "/v1/organizations/{org_id}/projects/{project_id}/gsc/sites",
            get(routes::gsc_routes::sites),
        )
        .route(
            "/v1/organizations/{org_id}/projects/{project_id}/gsc/site",
            post(routes::gsc_routes::set_site),
        )
        .route(
            "/v1/organizations/{org_id}/projects/{project_id}/gsc",
            axum::routing::delete(routes::gsc_routes::disconnect),
        )
        .route(
            "/v1/organizations/{org_id}/events/summary",
            get(routes::resource_routes::events_summary),
        )
        .route(
            "/v1/organizations/{org_id}/projects/{project_id}/competitors",
            get(routes::competitor_routes::list_competitors)
                .post(routes::competitor_routes::create_competitor),
        )
        .route(
            "/v1/organizations/{org_id}/competitors/{competitor_id}",
            axum::routing::delete(routes::competitor_routes::delete_competitor),
        )
        .route(
            "/v1/organizations/{org_id}/competitors/{competitor_id}/backlinks",
            get(routes::competitor_routes::list_competitor_backlinks),
        )
        .route(
            "/v1/organizations/{org_id}/competitors/{competitor_id}/backlinks/import",
            post(routes::competitor_routes::import_competitor_backlinks),
        )
        .route(
            "/v1/organizations/{org_id}/projects/{project_id}/backlink-gap",
            get(routes::competitor_routes::backlink_gap),
        )
        .route(
            "/v1/organizations/{org_id}/projects/{project_id}/backlink-graph",
            get(routes::graph_routes::backlink_graph),
        )
        .route(
            "/v1/organizations/{org_id}/projects/{project_id}/discovery/seeds",
            get(routes::discovery_routes::list_seeds).post(routes::discovery_routes::import_seeds),
        )
        .route(
            "/v1/organizations/{org_id}/projects/{project_id}/discovery/run",
            post(routes::discovery_routes::run_discovery),
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
            "/v1/organizations/{org_id}/alert-destinations",
            get(routes::alerts_routes::list_destinations)
                .post(routes::alerts_routes::create_destination),
        )
        .route(
            "/v1/organizations/{org_id}/alert-destinations/{dest_id}",
            axum::routing::patch(routes::alerts_routes::update_destination)
                .delete(routes::alerts_routes::delete_destination),
        )
        .route(
            "/v1/organizations/{org_id}/alert-destinations/{dest_id}/test",
            post(routes::alerts_routes::test_destination),
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
            "/v1/organizations/{org_id}/billing",
            get(routes::billing_routes::billing_status),
        )
        .route(
            "/v1/organizations/{org_id}/billing/usage",
            get(routes::billing_routes::billing_usage),
        )
        .route(
            "/v1/organizations/{org_id}/billing/checkout",
            post(routes::billing_routes::create_checkout),
        )
        .route(
            "/v1/organizations/{org_id}/billing/portal",
            post(routes::billing_routes::create_portal),
        )
        .route(
            "/v1/billing/webhook",
            post(routes::billing_routes::stripe_webhook),
        )
        // Public plan catalog (one source of truth for pricing/billing UIs).
        .route("/v1/plans", get(routes::billing_routes::list_plans))
        .route(
            "/v1/tools/backlink-check",
            post(routes::tools_routes::backlink_check),
        )
        // Pre-launch waitlist email capture (public, no session).
        .route("/v1/waitlist", post(routes::waitlist_routes::join))
        .route("/v1/waitlist/count", get(routes::waitlist_routes::count))
        .route(
            "/v1/waitlist/unsubscribe",
            post(routes::waitlist_routes::unsubscribe),
        )
        // Platform admin backoffice (migration 0022) — cross-tenant, gated on
        // users.is_platform_admin inside each handler.
        .route("/v1/admin/stats", get(routes::admin_routes::stats))
        .route(
            "/v1/admin/organizations",
            get(routes::admin_routes::list_organizations),
        )
        .route(
            "/v1/admin/organizations/{org_id}",
            axum::routing::delete(routes::admin_routes::delete_organization),
        )
        .route(
            "/v1/admin/organizations/{org_id}/suspend",
            post(routes::admin_routes::set_suspended),
        )
        .route(
            "/v1/admin/organizations/{org_id}/plan",
            post(routes::admin_routes::set_plan),
        )
        .route("/v1/admin/audit", get(routes::admin_routes::list_audit))
        .route("/v1/admin/users", get(routes::admin_routes::list_users))
        .route(
            "/v1/admin/users/{user_id}/admin",
            post(routes::admin_routes::set_admin),
        )
        // Founder-ops read models (docs/19): queue + monitoring + email, revenue,
        // plan-limit usage. GET-only, gated on is_platform_admin like the rest.
        .route("/v1/admin/ops", get(routes::admin_routes::ops))
        .route("/v1/admin/billing", get(routes::admin_routes::billing))
        .route("/v1/admin/analytics", get(routes::admin_routes::analytics))
        .route("/v1/admin/waitlist", get(routes::admin_routes::waitlist))
        .route(
            "/v1/admin/waitlist/resend",
            post(routes::admin_routes::resend_waitlist_confirmations),
        )
        .route(
            "/v1/admin/waitlist/sync-audience",
            post(routes::admin_routes::sync_waitlist_audience),
        )
        .route("/v1/admin/feedback", get(routes::admin_routes::feedback))
        .route(
            "/v1/admin/feedback/{id}/status",
            post(routes::admin_routes::set_feedback_status),
        )
        .route(
            "/v1/admin/subscriptions",
            get(routes::admin_routes::list_subscriptions),
        )
        .route(
            "/v1/admin/usage/limits",
            get(routes::admin_routes::usage_limits),
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
