//! Typed identifiers. Each entity gets its own newtype over `Uuid` so a
//! `ProjectId` cannot be passed where a `BacklinkId` is expected — a whole
//! class of mix-up bugs becomes a compile error. IDs are **UUIDv7**
//! (time-ordered, so B-tree inserts stay local) and generated app-side
//! (docs/03 §1).

use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;
use uuid::Uuid;

macro_rules! typed_id {
    ($(#[$m:meta])* $name:ident) => {
        $(#[$m])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(Uuid);

        impl $name {
            /// A fresh, time-ordered v7 id.
            pub fn new() -> Self {
                $name(Uuid::now_v7())
            }

            /// Wrap an existing uuid (e.g. one read from the database).
            pub fn from_uuid(id: Uuid) -> Self {
                $name(id)
            }

            pub fn as_uuid(&self) -> Uuid {
                self.0
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                fmt::Display::fmt(&self.0, f)
            }
        }

        impl FromStr for $name {
            type Err = uuid::Error;
            fn from_str(s: &str) -> Result<Self, Self::Err> {
                Ok($name(Uuid::parse_str(s)?))
            }
        }

        impl From<Uuid> for $name {
            fn from(id: Uuid) -> Self {
                $name(id)
            }
        }
    };
}

typed_id!(
    /// Identifies a user (docs/03 §3.1).
    UserId
);
typed_id!(
    /// Identifies an organization / workspace — the tenant boundary (docs/05).
    OrganizationId
);
typed_id!(
    /// Identifies a client (docs/03 §3.3).
    ClientId
);
typed_id!(
    /// Identifies a project (docs/03 §3.3).
    ProjectId
);
typed_id!(
    /// Identifies a campaign (docs/03 §3.3).
    CampaignId
);
typed_id!(
    /// Identifies a publisher (docs/03 §3.3).
    PublisherId
);
typed_id!(
    /// Identifies a backlink — the core entity (docs/03 §3.4).
    BacklinkId
);
typed_id!(
    /// Identifies a queued job (docs/07).
    JobId
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_v7_and_time_ordered() {
        let a = BacklinkId::new();
        std::thread::sleep(std::time::Duration::from_millis(2));
        let b = BacklinkId::new();
        assert_eq!(a.as_uuid().get_version_num(), 7, "must be UUIDv7");
        assert!(
            a < b,
            "v7 ids created later must sort later (B-tree locality, docs/03 §1)"
        );
    }

    #[test]
    fn roundtrips_through_string() {
        let id = OrganizationId::new();
        let parsed: OrganizationId = id.to_string().parse().unwrap();
        assert_eq!(id, parsed);
    }

    #[test]
    fn distinct_types_do_not_unify() {
        // This is a compile-time property, exercised here for documentation:
        // the following would not compile —
        //     let _: ProjectId = BacklinkId::new();
        // Each id is a distinct type over Uuid, so cross-entity mix-ups are
        // caught by the compiler, not at runtime.
        let p = ProjectId::new();
        let b = BacklinkId::new();
        assert_ne!(p.as_uuid(), b.as_uuid());
    }

    #[test]
    fn serde_is_transparent() {
        let id = UserId::new();
        let json = serde_json::to_string(&id).unwrap();
        // Serializes as a bare uuid string, not a wrapper object.
        assert_eq!(json, format!("\"{id}\""));
        let back: UserId = serde_json::from_str(&json).unwrap();
        assert_eq!(id, back);
    }
}
