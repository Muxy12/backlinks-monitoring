//! Google Search Console OAuth + connection management (Post-Link Impact BETA).
//!
//! Env-gated (inert-until-configured, like Turnstile/Stripe): without
//! `GOOGLE_OAUTH_CLIENT_ID` / `GOOGLE_OAUTH_CLIENT_SECRET` /
//! `GOOGLE_OAUTH_REDIRECT_URI` / `GSC_TOKEN_KEY` the endpoints 404 and the UI
//! shows "connect". Refresh tokens are stored AES-256-GCM-encrypted (`auth::
//! encrypt_secret`); the DB only ever holds ciphertext.
//!
//! The OAuth callback lands on the API origin with NO session cookie (the session
//! is first-party on the web origin), so the authenticated `auth-url` endpoint
//! seals `org|project|exp` into the `state` with the server key; the callback
//! decrypts it to recover the tenant — the signed state IS the authorization.

use crate::error::ApiError;
use crate::session::{check_csrf, current};
use crate::state::AppState;
use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::Redirect;
use axum_extra::extract::cookie::CookieJar;
use common::Role;
use db::authorize_org;
use db::repositories::gsc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

const SCOPE: &str = "https://www.googleapis.com/auth/webmasters.readonly";
const AUTH_URL: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
const SITES_URL: &str = "https://www.googleapis.com/webmasters/v3/sites";
const STATE_TTL_SECS: i64 = 900;

struct OAuthEnv {
    client_id: String,
    client_secret: String,
    redirect_uri: String,
}

fn oauth_env() -> Option<OAuthEnv> {
    let client_id = std::env::var("GOOGLE_OAUTH_CLIENT_ID")
        .ok()
        .filter(|s| !s.is_empty())?;
    let client_secret = std::env::var("GOOGLE_OAUTH_CLIENT_SECRET")
        .ok()
        .filter(|s| !s.is_empty())?;
    let redirect_uri = std::env::var("GOOGLE_OAUTH_REDIRECT_URI")
        .ok()
        .filter(|s| !s.is_empty())?;
    Some(OAuthEnv {
        client_id,
        client_secret,
        redirect_uri,
    })
}

fn token_key() -> Option<[u8; 32]> {
    let k = std::env::var("GSC_TOKEN_KEY")
        .ok()
        .filter(|s| !s.is_empty())?;
    auth::key_from_base64(&k).ok()
}

/// Whether the GSC integration is configured (surfaced to the UI via `me.features`).
pub fn configured() -> bool {
    oauth_env().is_some() && token_key().is_some()
}

/// Where to send the browser back to after the callback.
fn app_url() -> String {
    std::env::var("APP_URL")
        .ok()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "https://linkpulz.com".to_string())
}

fn seal_state(key: &[u8; 32], org: Uuid, project: Uuid) -> Result<String, ApiError> {
    let exp = chrono::Utc::now().timestamp() + STATE_TTL_SECS;
    let payload = format!("{org}|{project}|{exp}");
    auth::encrypt_secret_b64(key, payload.as_bytes()).map_err(|_| ApiError::Internal)
}

fn open_state(key: &[u8; 32], state: &str) -> Option<(Uuid, Uuid)> {
    let plain = auth::decrypt_secret_b64(key, state).ok()?;
    let s = String::from_utf8(plain).ok()?;
    let mut it = s.split('|');
    let org = Uuid::parse_str(it.next()?).ok()?;
    let project = Uuid::parse_str(it.next()?).ok()?;
    let exp: i64 = it.next()?.parse().ok()?;
    if chrono::Utc::now().timestamp() > exp {
        return None;
    }
    Some((org, project))
}

fn http_client() -> Result<reqwest::Client, ApiError> {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|_| ApiError::Internal)
}

// ─── GET auth-url ─────────────────────────────────────────────────────────────
#[derive(Serialize)]
pub struct AuthUrlView {
    pub url: String,
}

pub async fn auth_url(
    State(state): State<AppState>,
    Path((org_id, project_id)): Path<(Uuid, Uuid)>,
    jar: CookieJar,
) -> Result<Json<AuthUrlView>, ApiError> {
    let env = oauth_env().ok_or(ApiError::NotFound)?;
    let key = token_key().ok_or(ApiError::NotFound)?;
    let (user, _s) = current(&state, &jar).await?;
    let scope = authorize_org(&state.pool, user.id, org_id).await?;
    scope.require(Role::Member)?;
    db::repositories::projects::get(&state.pool, &scope, project_id).await?; // 404 if not ours

    let signed = seal_state(&key, org_id, project_id)?;
    let mut url = reqwest::Url::parse(AUTH_URL).map_err(|_| ApiError::Internal)?;
    url.query_pairs_mut()
        .append_pair("client_id", &env.client_id)
        .append_pair("redirect_uri", &env.redirect_uri)
        .append_pair("response_type", "code")
        .append_pair("scope", SCOPE)
        .append_pair("access_type", "offline")
        .append_pair("prompt", "consent")
        .append_pair("include_granted_scopes", "true")
        .append_pair("state", &signed);
    Ok(Json(AuthUrlView {
        url: url.to_string(),
    }))
}

// ─── GET oauth/callback (public — authorized by the signed state) ─────────────
#[derive(Deserialize)]
pub struct CallbackQuery {
    pub code: Option<String>,
    pub state: Option<String>,
    pub error: Option<String>,
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    refresh_token: Option<String>,
    expires_in: Option<i64>,
    scope: Option<String>,
}

pub async fn callback(State(state): State<AppState>, Query(q): Query<CallbackQuery>) -> Redirect {
    let back =
        |status: &str| Redirect::to(&format!("{}/settings/integrations?gsc={status}", app_url()));

    let Some(key) = token_key() else {
        return back("unavailable");
    };
    let Some(env) = oauth_env() else {
        return back("unavailable");
    };
    if q.error.is_some() {
        return back("denied");
    }
    let (Some(code), Some(state_param)) = (q.code, q.state) else {
        return back("error");
    };
    let Some((org_id, project_id)) = open_state(&key, &state_param) else {
        return back("error");
    };

    let Ok(client) = http_client() else {
        return back("error");
    };
    let form = [
        ("code", code.as_str()),
        ("client_id", env.client_id.as_str()),
        ("client_secret", env.client_secret.as_str()),
        ("redirect_uri", env.redirect_uri.as_str()),
        ("grant_type", "authorization_code"),
    ];
    let tok: TokenResponse = match client.post(TOKEN_URL).form(&form).send().await {
        Ok(r) => match r.json().await {
            Ok(t) => t,
            Err(e) => {
                tracing::warn!(error = %e, "gsc: token response unreadable");
                return back("error");
            }
        },
        Err(e) => {
            tracing::warn!(error = %e, "gsc: token exchange failed");
            return back("error");
        }
    };
    // A refresh token is only returned on first consent; prompt=consent forces it.
    let Some(refresh) = tok.refresh_token else {
        tracing::warn!("gsc: no refresh_token returned (already granted without prompt?)");
        return back("error");
    };
    let Ok(refresh_enc) = auth::encrypt_secret(&key, refresh.as_bytes()) else {
        return back("error");
    };
    let access_enc = auth::encrypt_secret(&key, tok.access_token.as_bytes()).ok();
    let access_expires = tok
        .expires_in
        .map(|s| chrono::Utc::now() + chrono::Duration::seconds(s));

    match gsc::upsert(
        &state.pool,
        Uuid::now_v7(),
        org_id,
        project_id,
        None,
        &refresh_enc,
        access_enc.as_deref(),
        access_expires,
        tok.scope.as_deref(),
    )
    .await
    {
        Ok(_) => back("connected"),
        Err(e) => {
            tracing::warn!(error = %e, "gsc: connection upsert failed");
            back("error")
        }
    }
}

// ─── GET status ───────────────────────────────────────────────────────────────
#[derive(Serialize)]
pub struct StatusView {
    pub configured: bool,
    pub connected: bool,
    pub site_url: Option<String>,
    pub google_email: Option<String>,
    pub last_sync_at: Option<String>,
}

pub async fn status(
    State(state): State<AppState>,
    Path((org_id, project_id)): Path<(Uuid, Uuid)>,
    jar: CookieJar,
) -> Result<Json<StatusView>, ApiError> {
    let (user, _s) = current(&state, &jar).await?;
    let scope = authorize_org(&state.pool, user.id, org_id).await?;
    let conn = gsc::get(&state.pool, &scope, project_id).await?;
    Ok(Json(StatusView {
        configured: configured(),
        connected: conn.is_some(),
        site_url: conn.as_ref().and_then(|c| c.site_url.clone()),
        google_email: conn.as_ref().and_then(|c| c.google_email.clone()),
        last_sync_at: conn.and_then(|c| c.last_sync_at.map(|d| d.to_rfc3339())),
    }))
}

// ─── GET sites (list the user's GSC properties) ───────────────────────────────
#[derive(Serialize)]
pub struct SiteEntry {
    pub site_url: String,
    pub permission_level: Option<String>,
}

/// Exchange the stored refresh token for a fresh access token (not persisted).
async fn fresh_access_token(
    client: &reqwest::Client,
    env: &OAuthEnv,
    key: &[u8; 32],
    refresh_enc: &[u8],
) -> Result<String, ApiError> {
    let refresh = auth::decrypt_secret(key, refresh_enc).map_err(|_| ApiError::Internal)?;
    let refresh = String::from_utf8(refresh).map_err(|_| ApiError::Internal)?;
    let form = [
        ("refresh_token", refresh.as_str()),
        ("client_id", env.client_id.as_str()),
        ("client_secret", env.client_secret.as_str()),
        ("grant_type", "refresh_token"),
    ];
    let tok: TokenResponse = client
        .post(TOKEN_URL)
        .form(&form)
        .send()
        .await
        .map_err(|_| ApiError::Internal)?
        .json()
        .await
        .map_err(|_| ApiError::Internal)?;
    Ok(tok.access_token)
}

pub async fn sites(
    State(state): State<AppState>,
    Path((org_id, project_id)): Path<(Uuid, Uuid)>,
    jar: CookieJar,
) -> Result<Json<Vec<SiteEntry>>, ApiError> {
    let env = oauth_env().ok_or(ApiError::NotFound)?;
    let key = token_key().ok_or(ApiError::NotFound)?;
    let (user, _s) = current(&state, &jar).await?;
    let scope = authorize_org(&state.pool, user.id, org_id).await?;
    scope.require(Role::Member)?;
    let conn = gsc::get(&state.pool, &scope, project_id)
        .await?
        .ok_or(ApiError::NotFound)?;

    let client = http_client()?;
    let access = fresh_access_token(&client, &env, &key, &conn.refresh_token_enc).await?;
    let payload: serde_json::Value = client
        .get(SITES_URL)
        .bearer_auth(access)
        .send()
        .await
        .map_err(|_| ApiError::Internal)?
        .json()
        .await
        .map_err(|_| ApiError::Internal)?;

    let entries = payload
        .get("siteEntry")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|e| {
                    Some(SiteEntry {
                        site_url: e.get("siteUrl")?.as_str()?.to_string(),
                        permission_level: e
                            .get("permissionLevel")
                            .and_then(|p| p.as_str())
                            .map(str::to_string),
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    Ok(Json(entries))
}

// ─── POST site (map a property) ───────────────────────────────────────────────
#[derive(Deserialize)]
pub struct SetSiteBody {
    pub site_url: String,
}

pub async fn set_site(
    State(state): State<AppState>,
    Path((org_id, project_id)): Path<(Uuid, Uuid)>,
    jar: CookieJar,
    headers: HeaderMap,
    Json(body): Json<SetSiteBody>,
) -> Result<StatusCode, ApiError> {
    let (user, session) = current(&state, &jar).await?;
    check_csrf(&jar, &headers, &session)?;
    let scope = authorize_org(&state.pool, user.id, org_id).await?;
    scope.require(Role::Member)?;
    let site = body.site_url.trim();
    if site.is_empty() {
        return Err(ApiError::Validation("site_url is required".into()));
    }
    if gsc::set_site(&state.pool, &scope, project_id, site).await? {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(ApiError::NotFound)
    }
}

// ─── DELETE (disconnect) ──────────────────────────────────────────────────────
pub async fn disconnect(
    State(state): State<AppState>,
    Path((org_id, project_id)): Path<(Uuid, Uuid)>,
    jar: CookieJar,
    headers: HeaderMap,
) -> Result<StatusCode, ApiError> {
    let (user, session) = current(&state, &jar).await?;
    check_csrf(&jar, &headers, &session)?;
    let scope = authorize_org(&state.pool, user.id, org_id).await?;
    scope.require(Role::Member)?;
    if gsc::delete(&state.pool, &scope, project_id).await? {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(ApiError::NotFound)
    }
}
