//! User feedback (migration 0033): a logged-in user submits a feature request
//! or a bug report, and can see their own past submissions. The platform admin
//! triages everything from the backoffice (see admin_routes::feedback).

use crate::error::ApiError;
use crate::session::{check_csrf, current};
use crate::state::AppState;
use axum::Json;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum_extra::extract::cookie::CookieJar;
use db::repositories::feedback;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

const KINDS: &[&str] = &["feature", "bug"];
const TITLE_MAX: usize = 160;
const BODY_MAX: usize = 4000;

#[derive(Debug, Deserialize)]
pub struct SubmitBody {
    /// "feature" or "bug".
    pub kind: String,
    pub title: String,
    pub body: String,
    /// Where the user was when submitting (optional, helps triage bugs).
    #[serde(default)]
    pub page_url: Option<String>,
    /// The workspace the user is viewing (optional, validated for membership).
    #[serde(default)]
    pub organization_id: Option<Uuid>,
}

#[derive(Debug, Serialize)]
pub struct FeedbackView {
    pub id: Uuid,
    pub kind: String,
    pub title: String,
    pub body: String,
    pub page_url: Option<String>,
    pub status: String,
    pub created_at: String,
    pub user_email: String,
}

impl From<feedback::Feedback> for FeedbackView {
    fn from(f: feedback::Feedback) -> Self {
        FeedbackView {
            id: f.id,
            kind: f.kind,
            title: f.title,
            body: f.body,
            page_url: f.page_url,
            status: f.status,
            created_at: f.created_at.to_rfc3339(),
            user_email: f.user_email,
        }
    }
}

/// POST /v1/feedback — submit a feature request or bug report.
pub async fn submit(
    State(state): State<AppState>,
    jar: CookieJar,
    headers: HeaderMap,
    Json(body): Json<SubmitBody>,
) -> Result<(StatusCode, Json<FeedbackView>), ApiError> {
    let (user, session) = current(&state, &jar).await?;
    check_csrf(&jar, &headers, &session)?;

    let kind = body.kind.trim();
    if !KINDS.contains(&kind) {
        return Err(ApiError::Validation("kind must be feature or bug".into()));
    }
    let title = body.title.trim();
    if title.is_empty() || title.chars().count() > TITLE_MAX {
        return Err(ApiError::Validation(format!(
            "title must be 1 to {TITLE_MAX} characters"
        )));
    }
    let text = body.body.trim();
    if text.is_empty() || text.chars().count() > BODY_MAX {
        return Err(ApiError::Validation(format!(
            "description must be 1 to {BODY_MAX} characters"
        )));
    }
    let page_url: Option<String> = body
        .page_url
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.chars().take(500).collect());

    // If a workspace is named, confirm the user belongs to it (never trust a
    // client-supplied org id); otherwise store no org.
    let organization_id = match body.organization_id {
        Some(org_id) => {
            db::authorize_org(&state.pool, user.id, org_id).await?;
            Some(org_id)
        }
        None => None,
    };

    let fb = feedback::create(
        &state.pool,
        feedback::NewFeedback {
            kind,
            title,
            body: text,
            page_url: page_url.as_deref(),
            user_id: user.id,
            user_email: &user.email,
            organization_id,
        },
    )
    .await?;
    Ok((StatusCode::CREATED, Json(fb.into())))
}

/// GET /v1/feedback/mine — the caller's own submissions, newest first.
pub async fn list_mine(
    State(state): State<AppState>,
    jar: CookieJar,
) -> Result<Json<Vec<FeedbackView>>, ApiError> {
    let (user, _session) = current(&state, &jar).await?;
    let rows = feedback::list_for_user(&state.pool, user.id, 50).await?;
    Ok(Json(rows.into_iter().map(FeedbackView::from).collect()))
}
