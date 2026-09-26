//! Container-runtime entrypoint (ADR-0002): binds 0.0.0.0:$PORT and serves
//! `api::app()`. Reads all config from the environment at runtime (never baked
//! in at build time, docs/01 §1.2).

use api::{ApiConfig, AppState};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let database_url = std::env::var("DATABASE_URL").map_err(|_| "DATABASE_URL is required")?;
    let max_conns = std::env::var("DB_MAX_CONNECTIONS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(8);

    let db = db::build_pool(&db::DbConfig::api(&database_url, max_conns, None)).await?;

    // Refuse to start against an unmigrated / newer-than-known schema (ADR-0005).
    // The pool uses the pooled endpoint; verify tolerates it (read-only).
    match db::migrate::verify(&db.pool).await {
        Ok(n) => tracing::info!(applied = n, "schema verified"),
        Err(e) => tracing::warn!(error = %e, "schema verify skipped/failed"),
    }

    let secure_cookies = std::env::var("APP_ENV").as_deref() == Ok("production");
    // Optional runtime config from the environment (never baked in). The webhook
    // secret enables `/v1/billing/webhook`; absent, that route 404s.
    let stripe_webhook_secret = std::env::var("STRIPE_WEBHOOK_SECRET")
        .ok()
        .filter(|s| !s.is_empty());
    let session_ttl_hours = std::env::var("SESSION_TTL_HOURS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(720);
    let cfg = ApiConfig {
        secure_cookies,
        stripe_webhook_secret,
        session_ttl_hours,
        ..ApiConfig::default()
    };
    // Bootstrap platform admins from ADMIN_EMAILS (comma-separated). The operator
    // signs up normally; matching accounts are promoted here at boot. No admin
    // credentials are ever committed or baked in (docs/01 §1.2).
    if let Ok(raw) = std::env::var("ADMIN_EMAILS") {
        let emails: Vec<String> = raw
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        if !emails.is_empty() {
            match db::repositories::users::promote_admins_by_email(&db.pool, &emails).await {
                Ok(n) => tracing::info!(promoted = n, configured = emails.len(), "platform admins ensured"),
                Err(e) => tracing::warn!(error = %e, "platform admin bootstrap failed"),
            }
        }
    }

    let state = AppState::new(db.pool, cfg);

    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8080);
    let listener = tokio::net::TcpListener::bind(("0.0.0.0", port)).await?;
    tracing::info!(port, "backlio api listening");

    axum::serve(listener, api::app(state)).await?;
    Ok(())
}
