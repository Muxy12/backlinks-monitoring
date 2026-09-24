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
