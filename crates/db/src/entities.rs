//! Row types for the identity/tenancy tables (docs/03 §3.1–§3.2). These are the
//! persistence shapes; the domain works in `common`/`core` types. Mapped by
//! name via `sqlx::FromRow`, so every query selects explicit columns.

use chrono::{DateTime, Utc};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    pub password_hash: String,
    pub name: String,
    pub email_verified_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl User {
    pub fn is_email_verified(&self) -> bool {
        self.email_verified_at.is_some()
    }
}

#[derive(Debug, Clone, FromRow)]
pub struct Organization {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow)]
pub struct Session {
    pub id: Uuid,
    pub user_id: Uuid,
    pub token_hash: Vec<u8>,
    pub csrf_token_hash: Vec<u8>,
    pub expires_at: DateTime<Utc>,
    pub revoked_at: Option<DateTime<Utc>>,
}

impl Session {
    pub fn is_active(&self, now: DateTime<Utc>) -> bool {
        self.revoked_at.is_none() && self.expires_at > now
    }
}

#[derive(Debug, Clone, FromRow)]
pub struct Client {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub name: String,
    pub company_name: Option<String>,
    pub website: Option<String>,
    pub contact_name: Option<String>,
    pub contact_email: Option<String>,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// A project row. `check_frequency_secs` is `default_check_frequency` read via
/// `EXTRACT(EPOCH ...)`, and `status` is the `project_status` enum read as text
/// — both cast in the query so no custom sqlx type mapping is needed.
#[derive(Debug, Clone, FromRow)]
pub struct Project {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub client_id: Option<Uuid>,
    pub name: String,
    pub domain: String,
    pub check_frequency_secs: i64,
    pub timezone: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub archived_at: Option<DateTime<Utc>>,
}

/// The core backlink row (docs/03 §3.4), read with `rel_type`/`status`/`cost`
/// cast to text/numeric-as-text so no custom sqlx type mapping is needed. This
/// is a create/list projection, not every column.
#[derive(Debug, Clone, FromRow)]
pub struct Backlink {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub project_id: Uuid,
    pub campaign_id: Option<Uuid>,
    pub publisher_id: Option<Uuid>,
    pub source_url: String,
    pub source_url_normalized: String,
    pub source_domain: String,
    pub target_url: String,
    pub target_url_normalized: String,
    pub target_domain: String,
    pub expected_anchor_text: Option<String>,
    pub expected_rel_type: String,
    pub status: String,
    pub cost: Option<String>,
    pub currency: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub next_check_at: Option<DateTime<Utc>>,
    pub last_checked_at: Option<DateTime<Utc>>,
}

/// The fields the state machine reads for one backlink (docs/08). Enum/array
/// columns are read as text so no custom sqlx mapping is needed.
#[derive(Debug, Clone, FromRow)]
pub struct MonitorState {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub status: String,
    pub expected_anchor_text: Option<String>,
    pub expected_rel_type: String,
    pub detected_anchor_text: Option<String>,
    pub detected_target_url: Option<String>,
    pub detected_rel_attributes: Option<Vec<String>>,
    pub detected_is_dofollow: Option<bool>,
    pub source_canonical: Option<String>,
    pub source_robots_noindex: bool,
    pub consecutive_failure_count: i32,
    pub last_seen_at: Option<DateTime<Utc>>,
    pub source_url: String,
    pub target_url: String,
}

/// A change event on a backlink's timeline (§30). `previous_data`/`current_data`
/// are JSON snapshots.
#[derive(Debug, Clone, FromRow)]
pub struct BacklinkEvent {
    pub id: Uuid,
    pub backlink_id: Uuid,
    pub event_type: String,
    pub previous_data: Option<serde_json::Value>,
    pub current_data: Option<serde_json::Value>,
    pub detected_at: DateTime<Utc>,
}

/// A campaign row (§22). `budget`/`status` are read as text so no custom sqlx
/// mapping is needed; dates are plain `NaiveDate`.
#[derive(Debug, Clone, FromRow)]
pub struct Campaign {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub project_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub status: String,
    pub start_date: Option<chrono::NaiveDate>,
    pub end_date: Option<chrono::NaiveDate>,
    pub target_backlinks: Option<i32>,
    pub budget: Option<String>,
    pub currency: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// A publisher row (§31) — the site a backlink is bought from.
#[derive(Debug, Clone, FromRow)]
pub struct Publisher {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub name: String,
    pub domain: String,
    pub contact_name: Option<String>,
    pub contact_email: Option<String>,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Scalar aggregates for a campaign/publisher rollup (§22, §31), computed on
/// read (docs/03 §3.3). Spend is fetched separately, grouped by currency, and
/// never summed across currencies. `avg_lifespan_days` is the average time a
/// now-lost link stayed live (`last_seen_at - created_at`); `None` when no link
/// has been lost yet.
#[derive(Debug, Clone, FromRow)]
pub struct RollupCounts {
    pub total: i64,
    pub live: i64,
    pub lost: i64,
    pub attention: i64,
    pub avg_lifespan_days: Option<f64>,
}
