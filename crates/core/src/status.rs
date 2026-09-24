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
            BacklinkStatus::Redirected => 9,
            BacklinkStatus::Lost => 8,
            BacklinkStatus::SourceError => 7,
            BacklinkStatus::TargetError => 6,
            BacklinkStatus::Changed => 5,
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
