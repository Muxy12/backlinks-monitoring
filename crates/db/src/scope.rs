//! Tenant scope — the ADR-0004 mechanism. `OrgScope` is proof that the current
//! actor is a member of an organization, and it is the only key that opens the
//! tenant-scoped repositories. It has **no public constructor**: the sole way to
//! obtain one is [`authorize_org`], which performs the membership lookup. This
//! turns "never trust organization_id from the frontend" (§18) into a type
//! property rather than a code-review checklist item.

use crate::error::DbError;
use common::Role;
use sqlx::PgPool;
use uuid::Uuid;

/// A verified tenant context: the org the actor may act in, and their role.
/// Cannot be deserialized, cloned from a raw id, or built by a caller — only
/// [`authorize_org`] mints it.
#[derive(Debug, Clone, Copy)]
pub struct OrgScope {
    organization_id: Uuid,
    role: Role,
}

impl OrgScope {
    pub fn organization_id(&self) -> Uuid {
        self.organization_id
    }

    pub fn role(&self) -> Role {
        self.role
    }

    /// Require at least `needed`, else `Forbidden`. Scope answers "which org";
    /// this answers "may they" (docs/05 §2).
    pub fn require(&self, needed: Role) -> Result<(), DbError> {
        if self.role.allows(needed) {
            Ok(())
        } else {
            Err(DbError::Forbidden)
        }
    }
}

/// Verify membership and mint an [`OrgScope`]. Returns `NotFound` (never
/// `Forbidden`) when the user is not a member, so a non-member cannot even
/// distinguish "org does not exist" from "you are not in it" (docs/05 §3, IDOR).
/// Membership is read fresh every call — removing a user takes effect on their
/// next request, not their next login (docs/05 §3 rule 5).
pub async fn authorize_org(
    pool: &PgPool,
    user_id: Uuid,
    organization_id: Uuid,
) -> Result<OrgScope, DbError> {
    let row: Option<(String,)> = sqlx::query_as(
        "SELECT role::text FROM organization_members \
         WHERE organization_id = $1 AND user_id = $2",
    )
    .bind(organization_id)
    .bind(user_id)
    .fetch_optional(pool)
    .await?;

    match row {
        None => Err(DbError::NotFound),
        Some((role,)) => {
            let role = role.parse::<Role>().map_err(|_| {
                DbError::Config(format!("unknown role {role:?} in organization_members"))
            })?;
            Ok(OrgScope {
                organization_id,
                role,
            })
        }
    }
}
