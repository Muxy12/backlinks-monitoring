//! Authentication primitives (docs/05, docs/11 S4). This increment is the pure
//! crypto surface: password hashing, opaque token generation/hashing, and CSRF
//! double-submit validation. Session and membership logic (the `OrgScope`
//! construction of ADR-0004) wire in with the repository layer, which needs
//! `db`.
//!
//! Principles enforced here:
//! - Passwords are **Argon2id** at an OWASP baseline (docs/10, S4).
//! - Tokens are random, and only their **SHA-256 hash** is ever stored, so a
//!   database dump is not a set of working credentials (S4, docs/03 §3.1).
//! - Comparisons of secret material are **constant-time**.

pub mod csrf;
pub mod error;
pub mod password;
pub mod token;
pub mod token_crypto;

pub use error::AuthError;
pub use password::{Argon2Params, hash_password, verify_password};
pub use token::{Token, hash_token, verify_token};
pub use token_crypto::{
    CryptoError, decrypt as decrypt_secret, decrypt_b64 as decrypt_secret_b64,
    encrypt as encrypt_secret, encrypt_b64 as encrypt_secret_b64, key_from_base64,
};
