//! Organization roles and the authorization gate (docs/05 §2). Scope answers
//! "which org"; role answers "may they". Both are required, neither substitutes
//! for the other (ADR-0004).

use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    Owner,
    Admin,
    Member,
    Viewer,
}

impl Role {
    /// Higher rank ⇒ more capability. Used for `require`.
    pub const fn rank(self) -> u8 {
        match self {
            Role::Owner => 3,
            Role::Admin => 2,
            Role::Member => 1,
            Role::Viewer => 0,
        }
    }

    /// True when this role is at least `needed` (docs/05 §2).
    pub fn allows(self, needed: Role) -> bool {
        self.rank() >= needed.rank()
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Role::Owner => "owner",
            Role::Admin => "admin",
            Role::Member => "member",
            Role::Viewer => "viewer",
        }
    }
}

impl fmt::Display for Role {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for Role {
    type Err = ();
    fn from_str(s: &str) -> Result<Role, ()> {
        Ok(match s {
            "owner" => Role::Owner,
            "admin" => Role::Admin,
            "member" => Role::Member,
            "viewer" => Role::Viewer,
            _ => return Err(()),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ranks_are_ordered() {
        assert!(Role::Owner.rank() > Role::Admin.rank());
        assert!(Role::Admin.rank() > Role::Member.rank());
        assert!(Role::Member.rank() > Role::Viewer.rank());
    }

    #[test]
    fn allows_is_at_least_semantics() {
        assert!(Role::Owner.allows(Role::Admin));
        assert!(Role::Admin.allows(Role::Admin));
        assert!(!Role::Member.allows(Role::Admin));
        assert!(Role::Viewer.allows(Role::Viewer));
        assert!(!Role::Viewer.allows(Role::Member));
    }

    #[test]
    fn string_round_trips() {
        for r in [Role::Owner, Role::Admin, Role::Member, Role::Viewer] {
            assert_eq!(r.as_str().parse::<Role>(), Ok(r));
        }
        assert_eq!("root".parse::<Role>(), Err(()));
    }
}
