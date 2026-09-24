//! Argon2id password hashing (docs/10, docs/11 S4).

use crate::error::AuthError;
use argon2::password_hash::phc::PasswordHash;
use argon2::password_hash::{PasswordHasher, PasswordVerifier};
use argon2::{Algorithm, Argon2, Params, Version};

/// Argon2id cost parameters. Defaults are the OWASP baseline (docs/10):
/// 19 MiB memory, 2 iterations, 1 lane. Tuned up on production hardware in
/// Phase 7; kept configurable so tests can use cheap params.
#[derive(Debug, Clone, Copy)]
pub struct Argon2Params {
    pub memory_kib: u32,
    pub iterations: u32,
    pub parallelism: u32,
}

impl Default for Argon2Params {
    fn default() -> Self {
        Argon2Params {
            memory_kib: 19456,
            iterations: 2,
            parallelism: 1,
        }
    }
}

impl Argon2Params {
    /// Cheap params for tests — do NOT use in production.
    #[doc(hidden)]
    pub fn insecure_fast() -> Self {
        Argon2Params {
            memory_kib: 64,
            iterations: 1,
            parallelism: 1,
        }
    }

    fn build(&self) -> Result<Argon2<'static>, AuthError> {
        let params = Params::new(self.memory_kib, self.iterations, self.parallelism, None)
            .map_err(|_| AuthError::HashingFailed)?;
        Ok(Argon2::new(Algorithm::Argon2id, Version::V0x13, params))
    }
}

/// Hash a password into a PHC string (`$argon2id$v=19$m=...$salt$hash`), which
/// embeds the salt and parameters, so a future parameter change still verifies
/// old hashes.
pub fn hash_password(password: &str, params: &Argon2Params) -> Result<String, AuthError> {
    // password-hash 0.6 auto-generates a large random salt via getrandom.
    let hash = params
        .build()?
        .hash_password(password.as_bytes())
        .map_err(|_| AuthError::HashingFailed)?;
    Ok(hash.to_string())
}

/// Verify a password against a stored PHC hash. Constant-time in the comparison
/// (argon2 recomputes and compares the derived key). Returns
/// `Err(InvalidCredentials)` on mismatch, `Err(MalformedHash)` if the stored
/// string is unparseable. The parameters are read from the hash string itself,
/// so this works regardless of the current [`Argon2Params`].
pub fn verify_password(password: &str, phc: &str) -> Result<(), AuthError> {
    let parsed = PasswordHash::new(phc).map_err(|_| AuthError::MalformedHash)?;
    Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .map_err(|_| AuthError::InvalidCredentials)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn params() -> Argon2Params {
        Argon2Params::insecure_fast()
    }

    #[test]
    fn hash_is_a_phc_argon2id_string_not_the_password() {
        let h = hash_password("correct horse battery staple", &params()).unwrap();
        assert!(h.starts_with("$argon2id$"), "must be Argon2id PHC: {h}");
        assert!(
            !h.contains("correct horse"),
            "hash must not contain the password"
        );
    }

    #[test]
    fn same_password_hashes_differently_each_time() {
        // Random salt -> different hash strings, both verifying.
        let a = hash_password("s3cret", &params()).unwrap();
        let b = hash_password("s3cret", &params()).unwrap();
        assert_ne!(a, b, "distinct salts must produce distinct hashes");
        assert!(verify_password("s3cret", &a).is_ok());
        assert!(verify_password("s3cret", &b).is_ok());
    }

    #[test]
    fn verify_accepts_correct_and_rejects_wrong() {
        let h = hash_password("hunter2", &params()).unwrap();
        assert!(verify_password("hunter2", &h).is_ok());
        assert!(matches!(
            verify_password("hunter3", &h),
            Err(AuthError::InvalidCredentials)
        ));
    }

    #[test]
    fn malformed_hash_is_distinguished_from_a_mismatch() {
        assert!(matches!(
            verify_password("x", "not-a-phc-string"),
            Err(AuthError::MalformedHash)
        ));
    }

    #[test]
    fn verify_reads_params_from_the_hash_so_old_hashes_still_work() {
        // Hash with one set of params, verify with the default verifier.
        let strong = Argon2Params {
            memory_kib: 256,
            iterations: 3,
            parallelism: 1,
        };
        let h = hash_password("portable", &strong).unwrap();
        assert!(verify_password("portable", &h).is_ok());
    }

    #[test]
    fn empty_password_is_handled() {
        let h = hash_password("", &params()).unwrap();
        assert!(verify_password("", &h).is_ok());
        assert!(verify_password("x", &h).is_err());
    }
}
