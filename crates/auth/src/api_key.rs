//! Subscriber API keys (`lp_live_…`) for the MCP server and keyed REST API
//! (docs/product/22 §1). A key is an opaque random token shown to the client
//! **once**; only its SHA-256 hash is stored and looked up (same scheme as
//! sessions — S4). `prefix` is a non-secret display id for the settings UI.

use crate::token::{Token, hash_token};

/// The public, human-visible marker on every key. Lets a leaked key be
/// recognised (and, later, auto-revoked) by its shape.
pub const KEY_PREFIX: &str = "lp_live_";
/// How many characters of the random part go into the displayable `prefix`.
const PREFIX_RANDOM_CHARS: usize = 6;

/// A freshly minted API key. `secret` is returned to the caller exactly once;
/// `prefix` and `hash` are what get stored.
pub struct ApiKey {
    /// The full key (`lp_live_<random>`). Show once, never persist.
    pub secret: String,
    /// Non-secret display id (`lp_live_<first chars>`), safe to store and show.
    pub prefix: String,
    /// SHA-256 of the full secret — the stored, looked-up value.
    pub hash: Vec<u8>,
}

/// Generate a new API key and its storage material.
pub fn generate_api_key() -> ApiKey {
    let t = Token::generate();
    let secret = format!("{KEY_PREFIX}{}", t.secret);
    let prefix = format!("{KEY_PREFIX}{}", &t.secret[..PREFIX_RANDOM_CHARS]);
    let hash = hash_token(&secret);
    ApiKey {
        secret,
        prefix,
        hash,
    }
}

/// SHA-256 of a presented key — hash the `Authorization: Bearer` value with this
/// and look the row up by it (an exact match on 256 bits of entropy).
pub fn hash_api_key(presented: &str) -> Vec<u8> {
    hash_token(presented)
}

/// Whether a presented string looks like one of our keys (cheap pre-filter
/// before touching the database).
pub fn looks_like_api_key(presented: &str) -> bool {
    presented.starts_with(KEY_PREFIX) && presented.len() > KEY_PREFIX.len() + PREFIX_RANDOM_CHARS
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_has_prefix_and_matching_hash() {
        let k = generate_api_key();
        assert!(k.secret.starts_with(KEY_PREFIX));
        assert!(k.prefix.starts_with(KEY_PREFIX));
        assert_eq!(k.hash.len(), 32);
        assert_eq!(k.hash, hash_api_key(&k.secret));
        // The prefix is a non-secret leading slice of the full secret.
        assert!(k.secret.starts_with(&k.prefix));
        assert!(k.prefix.len() < k.secret.len());
    }

    #[test]
    fn distinct_each_time() {
        assert_ne!(generate_api_key().secret, generate_api_key().secret);
    }

    #[test]
    fn looks_like_filters_obvious_non_keys() {
        assert!(looks_like_api_key(&generate_api_key().secret));
        assert!(!looks_like_api_key("Bearer nonsense"));
        assert!(!looks_like_api_key("lp_live_"));
        assert!(!looks_like_api_key(""));
    }
}
