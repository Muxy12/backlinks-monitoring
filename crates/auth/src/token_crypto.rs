//! Symmetric encryption for secrets we must store reversibly — specifically the
//! Google OAuth **refresh tokens** for the Search Console integration (docs: GSC
//! Post-Link Impact). Unlike session tokens (which we only ever hash), a refresh
//! token has to be decryptable to call Google later, so it is sealed with
//! **AES-256-GCM** and only the ciphertext is stored. The key comes from the
//! `GSC_TOKEN_KEY` environment variable (base64 of 32 bytes); absent key ⇒ the
//! whole integration is inert.
//!
//! Wire format of the stored blob: `nonce(12) || ciphertext || tag(16)`.

use base64::Engine;
use ring::aead::{AES_256_GCM, Aad, LessSafeKey, NONCE_LEN, Nonce, UnboundKey};
use ring::rand::{SecureRandom, SystemRandom};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CryptoError {
    #[error("encryption key must be 32 bytes (base64)")]
    BadKey,
    #[error("ciphertext is malformed")]
    BadCiphertext,
    #[error("encryption failed")]
    Seal,
    #[error("decryption failed")]
    Open,
}

/// Parse the base64 `GSC_TOKEN_KEY` into a 32-byte key. Accepts standard or
/// URL-safe base64 (with or without padding).
pub fn key_from_base64(s: &str) -> Result<[u8; 32], CryptoError> {
    let s = s.trim();
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(s)
        .or_else(|_| base64::engine::general_purpose::STANDARD_NO_PAD.decode(s))
        .or_else(|_| base64::engine::general_purpose::URL_SAFE.decode(s))
        .or_else(|_| base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(s))
        .map_err(|_| CryptoError::BadKey)?;
    let arr: [u8; 32] = bytes.try_into().map_err(|_| CryptoError::BadKey)?;
    Ok(arr)
}

/// Encrypt `plaintext` with AES-256-GCM, returning `nonce || ciphertext || tag`.
pub fn encrypt(key: &[u8; 32], plaintext: &[u8]) -> Result<Vec<u8>, CryptoError> {
    let unbound = UnboundKey::new(&AES_256_GCM, key).map_err(|_| CryptoError::BadKey)?;
    let sealing = LessSafeKey::new(unbound);

    let rng = SystemRandom::new();
    let mut nonce_bytes = [0u8; NONCE_LEN];
    rng.fill(&mut nonce_bytes).map_err(|_| CryptoError::Seal)?;
    let nonce = Nonce::assume_unique_for_key(nonce_bytes);

    let mut in_out = plaintext.to_vec();
    sealing
        .seal_in_place_append_tag(nonce, Aad::empty(), &mut in_out)
        .map_err(|_| CryptoError::Seal)?;

    let mut out = Vec::with_capacity(NONCE_LEN + in_out.len());
    out.extend_from_slice(&nonce_bytes);
    out.extend_from_slice(&in_out);
    Ok(out)
}

/// Encrypt and URL-safe-base64 (no padding) — convenient for an opaque,
/// tamper-proof token carried in a URL (e.g. an OAuth `state`).
pub fn encrypt_b64(key: &[u8; 32], plaintext: &[u8]) -> Result<String, CryptoError> {
    Ok(base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(encrypt(key, plaintext)?))
}

/// Inverse of [`encrypt_b64`].
pub fn decrypt_b64(key: &[u8; 32], s: &str) -> Result<Vec<u8>, CryptoError> {
    let raw = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(s.trim())
        .map_err(|_| CryptoError::BadCiphertext)?;
    decrypt(key, &raw)
}

/// Decrypt a blob produced by [`encrypt`].
pub fn decrypt(key: &[u8; 32], data: &[u8]) -> Result<Vec<u8>, CryptoError> {
    // At minimum: nonce (12) + tag (16).
    if data.len() < NONCE_LEN + 16 {
        return Err(CryptoError::BadCiphertext);
    }
    let unbound = UnboundKey::new(&AES_256_GCM, key).map_err(|_| CryptoError::BadKey)?;
    let opening = LessSafeKey::new(unbound);

    let (nonce_bytes, ciphertext) = data.split_at(NONCE_LEN);
    let nonce =
        Nonce::try_assume_unique_for_key(nonce_bytes).map_err(|_| CryptoError::BadCiphertext)?;

    let mut in_out = ciphertext.to_vec();
    let plain = opening
        .open_in_place(nonce, Aad::empty(), &mut in_out)
        .map_err(|_| CryptoError::Open)?;
    Ok(plain.to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key() -> [u8; 32] {
        [7u8; 32]
    }

    #[test]
    fn round_trip() {
        let k = key();
        let pt = b"1//0gRefreshTokenExample_abcDEF-123";
        let blob = encrypt(&k, pt).unwrap();
        assert_ne!(&blob[..], &pt[..], "stored blob is not plaintext");
        assert_eq!(decrypt(&k, &blob).unwrap(), pt);
    }

    #[test]
    fn nonce_is_random_so_ciphertext_differs() {
        let k = key();
        let a = encrypt(&k, b"same").unwrap();
        let b = encrypt(&k, b"same").unwrap();
        assert_ne!(a, b, "random nonce => different ciphertext each time");
    }

    #[test]
    fn wrong_key_fails() {
        let blob = encrypt(&key(), b"secret").unwrap();
        let mut other = key();
        other[0] ^= 0xFF;
        assert!(decrypt(&other, &blob).is_err());
    }

    #[test]
    fn tampered_ciphertext_fails() {
        let k = key();
        let mut blob = encrypt(&k, b"secret").unwrap();
        let last = blob.len() - 1;
        blob[last] ^= 0x01;
        assert!(decrypt(&k, &blob).is_err());
    }

    #[test]
    fn key_parsing() {
        let b64 = base64::engine::general_purpose::STANDARD.encode([9u8; 32]);
        assert_eq!(key_from_base64(&b64).unwrap(), [9u8; 32]);
        assert!(key_from_base64("too-short").is_err());
    }
}
