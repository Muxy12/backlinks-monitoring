//! Opaque tokens for sessions, email verification and password reset
//! (docs/03 §3.1, S4). A token is random bytes shown to the client **once**;
//! only its SHA-256 hash is stored. Lookup hashes the presented token and
//! compares in constant time, so the database never holds anything that can be
//! replayed.

use base64::Engine;
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;

/// 32 bytes of entropy — 256 bits, well beyond guessing.
const TOKEN_BYTES: usize = 32;

/// A freshly minted token: the `secret` goes to the client exactly once; the
/// `hash` (32 bytes) is what gets stored.
pub struct Token {
    /// URL-safe base64, no padding. Show to the client once, never persist.
    pub secret: String,
    /// SHA-256 of the secret. Persist this (`bytea`, 32 bytes).
    pub hash: Vec<u8>,
}

impl Token {
    /// Generate a new random token and its storage hash.
    pub fn generate() -> Token {
        let mut bytes = [0u8; TOKEN_BYTES];
        getrandom::fill(&mut bytes).expect("OS RNG must be available");
        let secret = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes);
        let hash = hash_token(&secret);
        Token { secret, hash }
    }
}

/// SHA-256 of a token secret — the value stored and looked up.
pub fn hash_token(secret: &str) -> Vec<u8> {
    let mut h = Sha256::new();
    h.update(secret.as_bytes());
    h.finalize().to_vec()
}

/// Constant-time check that a presented secret matches a stored hash. Used on
/// every session/token lookup so a timing side-channel cannot probe the store.
pub fn verify_token(presented_secret: &str, stored_hash: &[u8]) -> bool {
    let computed = hash_token(presented_secret);
    // Length guard first (ct_eq requires equal length); both are always 32.
    computed.len() == stored_hash.len() && computed.ct_eq(stored_hash).into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_hash_is_sha256_of_secret_and_32_bytes() {
        let t = Token::generate();
        assert_eq!(t.hash.len(), 32);
        assert_eq!(t.hash, hash_token(&t.secret));
    }

    #[test]
    fn secret_is_url_safe_and_high_entropy() {
        let t = Token::generate();
        // URL-safe base64 of 32 bytes, no padding => 43 chars, no + / = .
        assert_eq!(t.secret.len(), 43);
        assert!(!t.secret.contains(['+', '/', '=']));
    }

    #[test]
    fn distinct_tokens_each_time() {
        assert_ne!(Token::generate().secret, Token::generate().secret);
    }

    #[test]
    fn verify_matches_the_right_secret_only() {
        let t = Token::generate();
        assert!(verify_token(&t.secret, &t.hash));
        assert!(!verify_token("some-other-token", &t.hash));
    }

    #[test]
    fn verify_rejects_wrong_length_hash() {
        let t = Token::generate();
        assert!(!verify_token(&t.secret, &t.hash[..16]));
    }
}
