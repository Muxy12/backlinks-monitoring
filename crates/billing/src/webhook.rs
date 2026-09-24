//! Stripe webhook signature verification (§39, docs/11 S5). Implements Stripe's
//! scheme directly so it is pure and unit-testable: the signed payload is
//! `"{timestamp}.{body}"`, HMAC-SHA256'd with the endpoint secret; the header
//! carries `t=<unix>,v1=<hex>[,v1=<hex>...]`. We reject on a bad signature or a
//! timestamp outside the tolerance (replay protection).

use hmac::digest::KeyInit;
use hmac::{Hmac, Mac};
use sha2::Sha256;
use subtle::ConstantTimeEq;

type HmacSha256 = Hmac<Sha256>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum WebhookError {
    #[error("malformed signature header")]
    MalformedHeader,
    #[error("no matching signature")]
    NoMatch,
    #[error("timestamp outside tolerance (possible replay)")]
    TimestampOutOfTolerance,
    #[error("invalid webhook secret")]
    InvalidSecret,
}

/// Verify a Stripe-Signature header against the raw request body.
/// `now_unix` and `tolerance_secs` are arguments so the check is deterministic
/// in tests; the handler passes the real clock and e.g. 300s.
pub fn verify_signature(
    payload: &str,
    sig_header: &str,
    secret: &str,
    now_unix: i64,
    tolerance_secs: i64,
) -> Result<(), WebhookError> {
    let mut timestamp: Option<i64> = None;
    let mut v1_sigs: Vec<&str> = Vec::new();
    for part in sig_header.split(',') {
        let (k, v) = part.split_once('=').ok_or(WebhookError::MalformedHeader)?;
        match k.trim() {
            "t" => timestamp = v.trim().parse().ok(),
            "v1" => v1_sigs.push(v.trim()),
            _ => {}
        }
    }
    let ts = timestamp.ok_or(WebhookError::MalformedHeader)?;
    if v1_sigs.is_empty() {
        return Err(WebhookError::MalformedHeader);
    }

    // Replay protection: reject stale (or absurdly future) timestamps.
    if (now_unix - ts).abs() > tolerance_secs {
        return Err(WebhookError::TimestampOutOfTolerance);
    }

    let signed_payload = format!("{ts}.{payload}");
    let mut mac =
        HmacSha256::new_from_slice(secret.as_bytes()).map_err(|_| WebhookError::InvalidSecret)?;
    mac.update(signed_payload.as_bytes());
    let expected = mac.finalize().into_bytes();

    for sig in v1_sigs {
        if let Some(bytes) = decode_hex(sig)
            && bytes.len() == expected.len()
            && bytes.ct_eq(&expected).into()
        {
            return Ok(());
        }
    }
    Err(WebhookError::NoMatch)
}

fn decode_hex(s: &str) -> Option<Vec<u8>> {
    if s.len() % 2 != 0 {
        return None;
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).ok())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use hmac::digest::KeyInit;
    use hmac::{Hmac, Mac};
    use sha2::Sha256;

    fn sign(payload: &str, secret: &str, ts: i64) -> String {
        let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(format!("{ts}.{payload}").as_bytes());
        let hex: String = mac
            .finalize()
            .into_bytes()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        format!("t={ts},v1={hex}")
    }

    const SECRET: &str = "whsec_test_secret";
    const BODY: &str = r#"{"id":"evt_1","type":"invoice.paid"}"#;

    #[test]
    fn valid_signature_passes() {
        let header = sign(BODY, SECRET, 1000);
        assert_eq!(verify_signature(BODY, &header, SECRET, 1000, 300), Ok(()));
    }

    #[test]
    fn tampered_body_fails() {
        let header = sign(BODY, SECRET, 1000);
        assert_eq!(
            verify_signature("{\"id\":\"evt_evil\"}", &header, SECRET, 1000, 300),
            Err(WebhookError::NoMatch)
        );
    }

    #[test]
    fn wrong_secret_fails() {
        let header = sign(BODY, SECRET, 1000);
        assert_eq!(
            verify_signature(BODY, &header, "whsec_wrong", 1000, 300),
            Err(WebhookError::NoMatch)
        );
    }

    #[test]
    fn stale_timestamp_is_rejected() {
        let header = sign(BODY, SECRET, 1000);
        // 400s later, tolerance 300 -> replay rejected.
        assert_eq!(
            verify_signature(BODY, &header, SECRET, 1400, 300),
            Err(WebhookError::TimestampOutOfTolerance)
        );
    }

    #[test]
    fn multiple_v1_sigs_any_match_passes() {
        let ts = 2000;
        let good = sign(BODY, SECRET, ts);
        let good_hex = good.split("v1=").nth(1).unwrap();
        let header = format!("t={ts},v1=deadbeef,v1={good_hex}");
        assert_eq!(verify_signature(BODY, &header, SECRET, ts, 300), Ok(()));
    }

    #[test]
    fn malformed_header_fails() {
        assert_eq!(
            verify_signature(BODY, "garbage", SECRET, 1000, 300),
            Err(WebhookError::MalformedHeader)
        );
        assert_eq!(
            verify_signature(BODY, "t=1000", SECRET, 1000, 300),
            Err(WebhookError::MalformedHeader)
        );
    }
}
