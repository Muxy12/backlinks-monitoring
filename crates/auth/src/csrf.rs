//! CSRF protection: double-submit token bound to the session (docs/09 §1.1,
//! docs/11 S6).
//!
//! The flow: at login we issue a CSRF token, send its value in a
//! (JS-readable) cookie, and store its **hash** on the session row. Every
//! state-changing request must echo the token in an `X-CSRF-Token` header. We
//! accept the request only when the header matches the cookie **and** hashes to
//! the value bound to the session — so a token minted for another session is
//! rejected even if an attacker can set a cookie.

use crate::token::{Token, verify_token};
use subtle::ConstantTimeEq;

/// Issue a new CSRF token. `secret` goes in the cookie; `hash` is stored on the
/// session row (`sessions.csrf_token_hash`).
pub fn issue() -> Token {
    Token::generate()
}

/// Validate a state-changing request.
///
/// - `cookie_value`: the CSRF token from the request cookie.
/// - `header_value`: the token echoed in `X-CSRF-Token`.
/// - `session_csrf_hash`: the hash stored on the session row.
///
/// All comparisons are constant-time. Returns true only when the header equals
/// the cookie *and* is the token bound to this session.
pub fn validate(cookie_value: &str, header_value: &str, session_csrf_hash: &[u8]) -> bool {
    // Header must equal cookie (double-submit), constant-time.
    let cookie_b = cookie_value.as_bytes();
    let header_b = header_value.as_bytes();
    let double_submit_ok = cookie_b.len() == header_b.len() && cookie_b.ct_eq(header_b).into();

    // ...and the presented token must be the one bound to the session.
    let bound_ok = verify_token(header_value, session_csrf_hash);

    double_submit_ok && bound_ok
}

/// Convenience for tests / callers that only have the header + session hash
/// (e.g. when the cookie is validated elsewhere).
pub fn matches_session(header_value: &str, session_csrf_hash: &[u8]) -> bool {
    verify_token(header_value, session_csrf_hash)
}

/// Re-export so callers can compute the stored hash when persisting a session.
pub use crate::token::hash_token as csrf_hash;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_double_submit_bound_to_session_passes() {
        let t = issue();
        // Cookie == header == the issued secret; session stores its hash.
        assert!(validate(&t.secret, &t.secret, &t.hash));
    }

    #[test]
    fn header_not_matching_cookie_fails() {
        let t = issue();
        let other = issue();
        assert!(!validate(&t.secret, &other.secret, &t.hash));
    }

    #[test]
    fn token_from_another_session_fails_even_if_cookie_and_header_agree() {
        // Attacker sets both cookie and header to a token they minted, but it is
        // not the token bound to the victim's session.
        let victim = issue();
        let attacker = issue();
        assert!(
            !validate(&attacker.secret, &attacker.secret, &victim.hash),
            "a token not bound to the session must be rejected (S6)"
        );
    }

    #[test]
    fn matches_session_checks_binding_only() {
        let t = issue();
        assert!(matches_session(&t.secret, &t.hash));
        assert!(!matches_session("forged", &t.hash));
    }

    #[test]
    fn empty_values_do_not_pass() {
        let t = issue();
        assert!(!validate("", "", &t.hash));
        assert!(!validate(&t.secret, &t.secret, b""));
    }
}
