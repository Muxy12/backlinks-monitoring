//! Forum / community detection (Spot Finder, Lot E). Pure: a URL in, a forum
//! classification out — no I/O. Used to split the SERP results into "forums &
//! communities" (places to participate / earn contextual links) vs the
//! editorial/buyable spots.
//!
//! Detection is by well-known URL footprints of forum software and large
//! community platforms. It is deliberately conservative: a miss just means a URL
//! isn't offered as a community lead, never a wrong action.

/// The kind of community a URL looks like.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ForumKind {
    PhpBB,
    VBulletin,
    XenForo,
    Discourse,
    Reddit,
    /// A forum/community by a generic path marker (`/forum/`, `/community/`…).
    Generic,
}

impl ForumKind {
    pub fn as_str(self) -> &'static str {
        match self {
            ForumKind::PhpBB => "phpbb",
            ForumKind::VBulletin => "vbulletin",
            ForumKind::XenForo => "xenforo",
            ForumKind::Discourse => "discourse",
            ForumKind::Reddit => "reddit",
            ForumKind::Generic => "generic",
        }
    }
}

/// Classify a URL as a forum/community page, or `None` if it doesn't look like
/// one. Matching is case-insensitive on the lowercased URL.
pub fn forum_kind(url: &str) -> Option<ForumKind> {
    let u = url.to_ascii_lowercase();

    // Large community platforms by host.
    if u.contains("reddit.com/r/") {
        return Some(ForumKind::Reddit);
    }

    // Forum-software footprints (URL shapes these engines produce).
    if u.contains("/viewtopic.php") || u.contains("/viewforum.php") {
        return Some(ForumKind::PhpBB);
    }
    if u.contains("/showthread.php") || u.contains("/forumdisplay.php") {
        return Some(ForumKind::VBulletin);
    }
    if u.contains("/threads/") || u.contains("/forums/") {
        return Some(ForumKind::XenForo);
    }
    // Discourse threads live under /t/<slug>/<id>.
    if u.contains("/t/") && u.contains("discourse") {
        return Some(ForumKind::Discourse);
    }

    // Generic path markers (strong hint of a forum/community section).
    for marker in [
        "/forum/",
        "/forums",
        "/community/",
        "/community",
        "/discussion",
        "/board/",
    ] {
        if u.contains(marker) {
            return Some(ForumKind::Generic);
        }
    }
    None
}

/// Whether a URL looks like a forum/community page.
pub fn is_forum(url: &str) -> bool {
    forum_kind(url).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_known_footprints() {
        assert_eq!(
            forum_kind("https://x.com/viewtopic.php?t=9"),
            Some(ForumKind::PhpBB)
        );
        assert_eq!(
            forum_kind("https://x.com/showthread.php?t=9"),
            Some(ForumKind::VBulletin)
        );
        assert_eq!(
            forum_kind("https://x.com/threads/some-topic.123/"),
            Some(ForumKind::XenForo)
        );
        assert_eq!(
            forum_kind("https://meta.discourse.org/t/topic/42"),
            Some(ForumKind::Discourse)
        );
        assert_eq!(
            forum_kind("https://www.reddit.com/r/SEO/comments/x"),
            Some(ForumKind::Reddit)
        );
        assert_eq!(
            forum_kind("https://site.com/forum/thread-1"),
            Some(ForumKind::Generic)
        );
        assert_eq!(
            forum_kind("https://site.com/community/"),
            Some(ForumKind::Generic)
        );
    }

    #[test]
    fn non_forum_urls_are_none() {
        assert!(forum_kind("https://blog.com/best-seo-tools").is_none());
        assert!(forum_kind("https://news.com/article/123").is_none());
        assert!(!is_forum("https://shop.com/product/42"));
    }

    #[test]
    fn matching_is_case_insensitive() {
        assert!(is_forum("https://Site.com/Forum/Thread"));
    }
}
