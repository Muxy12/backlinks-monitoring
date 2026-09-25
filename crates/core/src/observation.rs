//! What a single check tells us — classified **before** the state machine sees
//! it (docs/08 §1). This is where the critical distinction is made, by the type
//! system rather than by an `if` chain someone can later get wrong:
//!
//! - `Absent` is the ONLY variant that can lead to `lost`.
//! - Everything else — timeout, block, error, redirect — is a different fact.

use crate::status::RelSet;
use serde::{Deserialize, Serialize};

/// What we saw about the link and its page on one successful fetch+parse.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Detected {
    /// The matched anchor text, whitespace-normalized. `None` if the anchor
    /// wraps only an image with no alt (docs/06 §3.2 renders that as a marker
    /// upstream; here it is simply the text we will compare).
    pub anchor_text: Option<String>,
    /// The normalized href the link points at.
    pub target_url: Option<String>,
    /// The link's `rel` attribute set.
    pub rel: RelSet,
    /// Whether the link is dofollow, already accounting for page-level nofollow
    /// (computed by the crawler, docs/06 §3.3).
    pub is_dofollow: bool,
    /// Source page canonical, if present.
    pub canonical: Option<String>,
    /// Source page carries `noindex` (meta robots or `X-Robots-Tag`).
    pub noindex: bool,
    /// The link's nature: `true` when the anchor wraps only an image (docs/06
    /// §3.2), `false` for a text link.
    pub is_image: bool,
    /// When `is_image`, the image's `alt` (may be empty); `None` for text links.
    pub image_alt: Option<String>,
}

/// Why a connection could not be made or completed. We learned nothing about
/// the link from any of these.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConnErrorKind {
    Timeout,
    Dns,
    ConnectionRefused,
    Tls,
    TooManyRedirects,
    Other,
}

/// Why we chose not to, or could not, look — as opposed to looking and finding
/// nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BlockReason {
    RobotsTxt,
    SsrfGuard,
    OversizedBody,
    NonHtmlContent,
}

/// The classified result of one check. Exactly one of these per check.
/// Tagged `observation` (not `kind`, which collides with the `kind` field on
/// the `SourceUnreachable` variant).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "observation", rename_all = "snake_case")]
pub enum Observation {
    /// Page fetched, parsed, target link found.
    Present(Detected),
    /// Page fetched and parsed successfully; the target link is genuinely NOT
    /// in the document. **The only observation that can lead to `lost`.**
    Absent,
    /// Source returned 404/410 — the page is gone.
    SourceGone { status: u16 },
    /// Source returned another 4xx/5xx.
    SourceHttpError { status: u16 },
    /// Timeout, DNS failure, refused, TLS. We learned nothing.
    SourceUnreachable { kind: ConnErrorKind },
    /// robots.txt disallow, SSRF guard, oversized body, non-HTML.
    SourceBlocked { reason: BlockReason },
    /// Source resolved to a materially different URL.
    SourceRedirected { to: String, permanent: bool },
}

impl Observation {
    /// True only for `Absent`. Used by the property test that guarantees no
    /// other observation can ever produce `lost` (docs/08 §5).
    pub fn is_absent(&self) -> bool {
        matches!(self, Observation::Absent)
    }

    /// True when we successfully saw the link present.
    pub fn is_present(&self) -> bool {
        matches!(self, Observation::Present(_))
    }
}
