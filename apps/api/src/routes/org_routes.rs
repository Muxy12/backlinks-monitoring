//! Organization routes (docs/09 §Organizations): create and list. Creating an
//! org is a state-changing, authenticated action, so it requires a valid
//! session and passes the CSRF check (S6).

use crate::error::ApiError;
use crate::session::{check_csrf, current};
use crate::state::AppState;
use axum::Json;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum_extra::extract::cookie::CookieJar;
use db::repositories::orgs;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct CreateOrgBody {
    pub name: String,
    /// Optional URL slug; derived from the name when omitted.
    pub slug: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct OrgView {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub role: String,
}

/// Turn a name into a slug candidate: lowercase, non-alphanumerics to hyphens,
/// trimmed, bounded. Matches the `organizations_slug_format` CHECK
/// (migrations/0003) shape.
fn slugify(input: &str) -> String {
    let mut out = String::new();
    let mut prev_hyphen = false;
    for ch in input.trim().to_lowercase().chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch);
            prev_hyphen = false;
        } else if !prev_hyphen && !out.is_empty() {
            out.push('-');
            prev_hyphen = true;
        }
    }
    let s = out.trim_matches('-').to_string();
    s.chars()
        .take(63)
        .collect::<String>()
        .trim_matches('-')
        .to_string()
}

pub async fn create(
    State(state): State<AppState>,
    jar: CookieJar,
    headers: HeaderMap,
    Json(body): Json<CreateOrgBody>,
) -> Result<(StatusCode, Json<OrgView>), ApiError> {
    let (user, session) = current(&state, &jar).await?;
    check_csrf(&jar, &headers, &session)?;

    let name = body.name.trim();
    if name.is_empty() || name.len() > 200 {
        return Err(ApiError::Validation("name must be 1–200 characters".into()));
    }
    let slug = body
        .slug
        .map(|s| slugify(&s))
        .unwrap_or_else(|| slugify(name));
    if slug.len() < 3 {
        return Err(ApiError::Validation(
            "workspace URL must be at least 3 characters (letters/numbers)".into(),
        ));
    }

    let org = orgs::create_with_owner(&state.pool, Uuid::now_v7(), name, &slug, user.id).await?;
    // Start the 10-day free trial (no card) for the new workspace (docs/20).
    db::repositories::billing::start_trial(&state.pool, org.id).await?;
    // Email alerts ON by default: seed the owner's address as an enabled email
    // destination so a lost-link alert reaches their inbox out of the box. They
    // can edit or remove it in Settings → Notifications. Best-effort.
    if let Ok(scope) = db::authorize_org(&state.pool, user.id, org.id).await {
        let _ = db::repositories::alerts::create(
            &state.pool,
            &scope,
            Uuid::now_v7(),
            "email",
            &user.email,
            "My email",
            "immediate",
        )
        .await;
    }
    // Funnel: trial started (best-effort — docs/20 §P8).
    if let Err(e) = db::repositories::analytics::record(
        &state.pool,
        "trial_started",
        Some(user.id),
        Some(org.id),
        None,
        serde_json::json!({}),
    )
    .await
    {
        tracing::warn!(error = %e, "failed to record trial_started analytics event");
    }
    // Welcome / trial-started email (best-effort + idempotent — docs/20 trial UX).
    if let Ok(true) = db::repositories::trial_emails::claim(&state.pool, org.id, "started").await {
        let _ = notifications::trial::send(&user.email, name, "started", 10).await;
    }
    Ok((
        StatusCode::CREATED,
        Json(OrgView {
            id: org.id,
            name: org.name,
            slug: org.slug,
            role: "owner".into(),
        }),
    ))
}

pub async fn list(
    State(state): State<AppState>,
    jar: CookieJar,
) -> Result<Json<Vec<OrgView>>, ApiError> {
    let (user, _session) = current(&state, &jar).await?;
    let orgs = orgs::list_for_user(&state.pool, user.id).await?;
    Ok(Json(
        orgs.into_iter()
            .map(|(o, role)| OrgView {
                id: o.id,
                name: o.name,
                slug: o.slug,
                role: role.to_string(),
            })
            .collect(),
    ))
}

#[cfg(test)]
mod tests {
    use super::slugify;

    #[test]
    fn slugify_produces_valid_slugs() {
        assert_eq!(slugify("Acme SEO Agency"), "acme-seo-agency");
        assert_eq!(slugify("  Multiple   Spaces  "), "multiple-spaces");
        assert_eq!(slugify("Weird!!!Chars@@@Here"), "weird-chars-here");
        assert_eq!(slugify("--leading-trailing--"), "leading-trailing");
    }
}
