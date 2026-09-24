//! Auth error type. Deliberately coarse on the failure paths so callers cannot
//! accidentally leak *why* a credential check failed (S4 — uniform failures).

use thiserror::Error;

#[derive(Debug, Error)]
pub enum AuthError {
    /// Password did not match, or the stored hash was unreadable. Callers must
    /// present this uniformly with "user not found" so login/reset never reveal
    /// whether an email exists (docs/09 §2, S4).
    #[error("invalid credentials")]
    InvalidCredentials,

    /// A stored hash string could not be parsed. Internal; never shown to users.
    #[error("password hash is malformed")]
    MalformedHash,

    /// Hashing failed (e.g. bad parameters). Internal.
    #[error("password hashing failed")]
    HashingFailed,
}
