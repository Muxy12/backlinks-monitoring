//! Backlink status, rel attributes, and the follow/dofollow rules.

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// The ten states a backlink can hold (docs/08 §2). `archived` is deliberately
/// **not** here: archival lives in `backlinks.archived_at` so an archived link
/// keeps its last known state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BacklinkStatus {
    Pending,
    Live,
    /// Page fetched OK but the link was absent — not yet confirmed lost (§24).
    SuspectedLost,
    Lost,
    Changed,
    Nofollow,
    Redirected,
    SourceError,
    TargetError,
    Blocked,
    Unknown,
}

impl BacklinkStatus {
    /// Precedence rank when several conditions hold at once (docs/08 §2.1).
    /// Higher wins the single `status` field. The full picture is never lost —
    /// events are emitted per condition regardless of which one wins here.
    ///
    /// ```text
    /// redirected > lost > source_error > target_error
    ///            > changed > nofollow > blocked > unknown > live > pending
    /// ```
    fn rank(self) -> u8 {
        match self {
            BacklinkStatus::Redirected => 10,
            BacklinkStatus::Lost => 9,
            BacklinkStatus::SourceError => 8,
            BacklinkStatus::TargetError => 7,
            BacklinkStatus::Changed => 6,
            BacklinkStatus::SuspectedLost => 5,
            BacklinkStatus::Nofollow => 4,
            BacklinkStatus::Blocked => 3,
            BacklinkStatus::Unknown => 2,
            BacklinkStatus::Live => 1,
            BacklinkStatus::Pending => 0,
        }
    }

    /// Whether this status should surface in the "needs attention" views
    /// (§34) and count toward Value at Risk (§33).
    pub fn needs_attention(self) -> bool {
        matches!(
            self,
            BacklinkStatus::Lost
                | BacklinkStatus::SuspectedLost
                | BacklinkStatus::Changed
                | BacklinkStatus::Nofollow
                | BacklinkStatus::Redirected
                | BacklinkStatus::SourceError
                | BacklinkStatus::TargetError
        )
    }

    /// `blocked` and `unknown` mean "we could not verify", not "it is broken".
    /// The UI colours them grey, not red (docs/14 §1.1), and they are counted
    /// separately from `lost`.
    pub fn is_unverifiable(self) -> bool {
        matches!(self, BacklinkStatus::Blocked | BacklinkStatus::Unknown)
    }

    /// The snake_case token stored in the `backlink_status` Postgres enum.
    pub fn as_str(self) -> &'static str {
        match self {
            BacklinkStatus::Pending => "pending",
            BacklinkStatus::Live => "live",
            BacklinkStatus::SuspectedLost => "suspected_lost",
            BacklinkStatus::Lost => "lost",
            BacklinkStatus::Changed => "changed",
            BacklinkStatus::Nofollow => "nofollow",
            BacklinkStatus::Redirected => "redirected",
            BacklinkStatus::SourceError => "source_error",
            BacklinkStatus::TargetError => "target_error",
            BacklinkStatus::Blocked => "blocked",
            BacklinkStatus::Unknown => "unknown",
        }
    }

    /// Parse from the database enum text.
    pub fn from_db(s: &str) -> Option<Self> {
        Some(match s {
            "pending" => BacklinkStatus::Pending,
            "live" => BacklinkStatus::Live,
            "suspected_lost" => BacklinkStatus::SuspectedLost,
            "lost" => BacklinkStatus::Lost,
            "changed" => BacklinkStatus::Changed,
            "nofollow" => BacklinkStatus::Nofollow,
            "redirected" => BacklinkStatus::Redirected,
            "source_error" => BacklinkStatus::SourceError,
            "target_error" => BacklinkStatus::TargetError,
            "blocked" => BacklinkStatus::Blocked,
            "unknown" => BacklinkStatus::Unknown,
            _ => return None,
        })
    }

    /// Pick the higher-precedence of two candidate statuses.
    pub fn max_precedence(self, other: BacklinkStatus) -> BacklinkStatus {
        if other.rank() > self.rank() {
            other
        } else {
            self
        }
    }
}

/// A single `rel` token we care about. Unknown tokens (`noopener`, `me`, …)
/// are not modelled — they do not affect follow semantics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RelType {
    Dofollow,
    Nofollow,
    Sponsored,
    Ugc,
}

impl RelType {
    pub fn as_str(self) -> &'static str {
        match self {
            RelType::Dofollow => "dofollow",
            RelType::Nofollow => "nofollow",
            RelType::Sponsored => "sponsored",
            RelType::Ugc => "ugc",
        }
    }

    pub fn from_db(s: &str) -> Option<Self> {
        Some(match s {
            "dofollow" => RelType::Dofollow,
            "nofollow" => RelType::Nofollow,
            "sponsored" => RelType::Sponsored,
            "ugc" => RelType::Ugc,
            _ => return None,
        })
    }
}

/// The set of `rel` attributes detected on a link. Ordered, so equality and
/// display are stable and a change from `nofollow` to `sponsored` is visible
/// rather than flattened to "still nofollow" (docs/06 §3.3).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RelSet(BTreeSet<RelType>);

impl RelSet {
    pub fn new() -> Self {
        RelSet(BTreeSet::new())
    }

    pub fn insert(&mut self, r: RelType) {
        self.0.insert(r);
    }

    pub fn contains(&self, r: RelType) -> bool {
        self.0.contains(&r)
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// The tokens that strip a link's follow signal.
    fn suppresses_follow(&self) -> bool {
        self.contains(RelType::Nofollow)
            || self.contains(RelType::Sponsored)
            || self.contains(RelType::Ugc)
    }

    /// Dofollow is the **absence** of nofollow/sponsored/ugc, not the presence
    /// of a `dofollow` token (there is no such HTML token), **and** the page
    /// must not itself be nofollow (docs/06 §3.3). `page_nofollow` comes from a
    /// page-level `<meta name="robots" content="nofollow">`.
    pub fn is_dofollow(&self, page_nofollow: bool) -> bool {
        !page_nofollow && !self.suppresses_follow()
    }

    /// Rebuild a set from stored tokens (e.g. `detected_rel_attributes`).
    /// Unknown tokens are ignored.
    pub fn from_tokens<I, S>(tokens: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        tokens
            .into_iter()
            .filter_map(|t| RelType::from_db(t.as_ref()))
            .collect()
    }

    /// Snapshot as sorted strings, for storage in `detected_rel_attributes`
    /// and for event payloads.
    pub fn to_sorted_vec(&self) -> Vec<String> {
        self.0
            .iter()
            .map(|r| match r {
                RelType::Dofollow => "dofollow",
                RelType::Nofollow => "nofollow",
                RelType::Sponsored => "sponsored",
                RelType::Ugc => "ugc",
            })
            .map(String::from)
            .collect()
    }
}

impl FromIterator<RelType> for RelSet {
    fn from_iter<I: IntoIterator<Item = RelType>>(iter: I) -> Self {
        RelSet(iter.into_iter().collect())
    }
}
