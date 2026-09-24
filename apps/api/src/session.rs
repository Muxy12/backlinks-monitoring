//! Cookie-based session helpers (docs/09 §1): issuing the session + CSRF
//! cookies, resolving the current user, and enforcing CSRF on state-changing
//! requests.

use crate::error::ApiError;
use crate::state::AppState;
use axum::http::HeaderMap;
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use chrono::Utc;
use db::entities::{Session, User};
use db::repositories::{sessions, users};

pub const SESSION_COOKIE: &str = "session";
pub const CSRF_COOKIE: &str = "csrf";
pub const CSRF_HEADER: &str = "x-csrf-token";

/// Add the session (HttpOnly) and CSRF (readable) cookies to the jar. The
/// session cookie is `HttpOnly; SameSite=Lax` and `Secure` in production
/// (docs/09 §1). Returns the updated jar for the handler to return.
pub fn issue_cookies(
    jar: CookieJar,
    session_secret: String,
    csrf_secret: String,
    secure: bool,
    ttl_hours: i64,
) -> CookieJar {
    let max_age = time::Duration::hours(ttl_hours);

    let mut session = Cookie::new(SESSION_COOKIE, session_secret);
    session.set_http_only(true);
    session.set_secure(secure);
    session.set_same_site(SameSite::Lax);
    session.set_path("/");
    session.set_max_age(max_age);

    // Readable by JS so the client can echo it in the X-CSRF-Token header.
    let mut csrf = Cookie::new(CSRF_COOKIE, csrf_secret);
    csrf.set_http_only(false);
    csrf.set_secure(secure);
    csrf.set_same_site(SameSite::Lax);
    csrf.set_path("/");
    csrf.set_max_age(max_age);

    jar.add(session).add(csrf)
}

/// Clear both cookies (logout).
pub fn clear_cookies(jar: CookieJar) -> CookieJar {
    let mut s = Cookie::new(SESSION_COOKIE, "");
    s.set_path("/");
    let mut c = Cookie::new(CSRF_COOKIE, "");
    c.set_path("/");
    jar.remove(s).remove(c)
}

/// Resolve the current user and session from the session cookie, or
/// `Unauthorized`. Membership is not checked here — that is `authorize_org`.
pub async fn current(state: &AppState, jar: &CookieJar) -> Result<(User, Session), ApiError> {
    let secret = jar.get(SESSION_COOKIE).map(|c| c.value().to_string());
    let Some(secret) = secret else {
        return Err(ApiError::Unauthorized);
    };
    let token_hash = auth::hash_token(&secret);
    let session = sessions::find_active_by_token_hash(&state.pool, &token_hash, Utc::now())
        .await
        .map_err(|_| ApiError::Unauthorized)?;
    let user = users::find_by_id(&state.pool, session.user_id)
        .await
        .map_err(|_| ApiError::Unauthorized)?;
    Ok((user, session))
}

/// Enforce CSRF on a state-changing request: the `X-CSRF-Token` header must
/// match the CSRF cookie and be the token bound to this session (docs/11 S6).
pub fn check_csrf(jar: &CookieJar, headers: &HeaderMap, session: &Session) -> Result<(), ApiError> {
    let cookie = jar
        .get(CSRF_COOKIE)
        .map(|c| c.value().to_string())
        .unwrap_or_default();
    let header = headers
        .get(CSRF_HEADER)
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default();
    if auth::csrf::validate(&cookie, header, &session.csrf_token_hash) {
        Ok(())
    } else {
        Err(ApiError::Forbidden)
    }
}
