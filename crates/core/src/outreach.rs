//! Templated outreach drafts (link-building campaigns, BETA).
//!
//! Given a prospect's context, render a plain, deterministic first-contact email
//! (subject + body) with the merge fields filled in. No LLM, no I/O — this is a
//! pure function so it is unit-tested and renders the same every time.
//!
//! It is deliberately the single seam for AI-generated outreach later: a future
//! iteration can replace the body of `render_draft` (or branch inside it) to call
//! a model, and neither the API endpoint nor the UI changes, because they only
//! ever see `{subject, body}`.

use crate::reclamation::ReclaimAngle;

/// Everything the template needs to personalize a draft. Missing optionals fall
/// back to neutral wording so a draft is always complete.
#[derive(Debug, Clone, Copy)]
pub struct OutreachContext<'a> {
    /// The prospect site we are reaching out to, e.g. `example.com`.
    pub prospect_domain: &'a str,
    /// Contact name, if known (greeting falls back to "there").
    pub contact_name: Option<&'a str>,
    /// Our page we would like a link to, if chosen.
    pub target_url: Option<&'a str>,
    /// The sender's name for the sign-off, if known.
    pub sender_name: Option<&'a str>,
    /// The campaign name, for internal context in the subject fallback.
    pub campaign_name: &'a str,
    /// When set, this is a reclamation prospect (a link we already had and lost)
    /// and the draft references the specific link instead of a cold pitch.
    pub reclaim: Option<ReclaimPitch<'a>>,
}

/// The specifics of a lost link, used to render a reclamation pitch that leads
/// with the receipts (which page linked us, with which anchor, when, and how it
/// was lost). Built by the API from the prospect's origin backlink.
#[derive(Debug, Clone, Copy)]
pub struct ReclaimPitch<'a> {
    pub angle: ReclaimAngle,
    /// The page that used to carry the link.
    pub source_url: &'a str,
    /// The anchor text the link used, if known.
    pub anchor: Option<&'a str>,
    /// When the loss was detected (e.g. "2026-10-01"), if known.
    pub lost_on: Option<&'a str>,
}

/// A rendered, ready-to-copy draft.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutreachDraft {
    pub subject: String,
    pub body: String,
}

/// Render a first-contact outreach draft. Deterministic. A reclamation prospect
/// (`ctx.reclaim` set) gets a warm "we had this link, can we get it back" pitch;
/// everything else gets the cold first-contact pitch.
pub fn render_draft(ctx: OutreachContext<'_>) -> OutreachDraft {
    let domain = ctx.prospect_domain.trim();
    let greeting = match ctx.contact_name.map(str::trim).filter(|s| !s.is_empty()) {
        Some(name) => format!("Hi {name},"),
        None => "Hi there,".to_string(),
    };
    let sign_off = match ctx.sender_name.map(str::trim).filter(|s| !s.is_empty()) {
        Some(name) => format!("Thanks for your time,\n{name}"),
        None => "Thanks for your time,".to_string(),
    };
    let target = ctx.target_url.map(str::trim).filter(|s| !s.is_empty());

    if let Some(pitch) = ctx.reclaim {
        return render_reclaim(&greeting, &sign_off, domain, target, pitch);
    }

    let subject = if domain.is_empty() {
        format!("Collaboration — {}", ctx.campaign_name.trim())
    } else {
        format!("Quick question about {domain}")
    };
    let site = if domain.is_empty() {
        "your site"
    } else {
        domain
    };

    // The ask: reference our target page when we have one, otherwise stay generic.
    let ask = match target {
        Some(url) => format!(
            "I've been following {site} and think our resource at {url} would be a \
genuinely useful addition for your readers. Would you be open to including a link \
to it where it fits naturally?",
        ),
        None => format!(
            "I've been following {site} and would love to explore a collaboration — \
I think there's a natural fit between what you publish and the resources we're \
building. Could I share a couple of ideas?",
        ),
    };

    let body = format!("{greeting}\n\n{ask}\n\n{sign_off}");
    OutreachDraft { subject, body }
}

/// Render the reclamation pitch — leads with the receipts, then an angle-specific
/// ask. Deterministic; no LLM.
fn render_reclaim(
    greeting: &str,
    sign_off: &str,
    domain: &str,
    target_url: Option<&str>,
    pitch: ReclaimPitch<'_>,
) -> OutreachDraft {
    let subject = if domain.is_empty() {
        "A quick note about a link to our site".to_string()
    } else {
        format!("Your link to us on {domain}")
    };

    let source = match pitch.source_url.trim() {
        s if !s.is_empty() => s,
        _ if !domain.is_empty() => domain,
        _ => "your site",
    };
    let target = target_url.unwrap_or("our site");
    let anchor_part = match pitch.anchor.map(str::trim).filter(|s| !s.is_empty()) {
        Some(a) => format!(" (anchor \"{a}\")"),
        None => String::new(),
    };
    let when_part = match pitch.lost_on.map(str::trim).filter(|s| !s.is_empty()) {
        Some(d) => format!(", around {d}"),
        None => String::new(),
    };
    let lost_phrase = match pitch.angle {
        ReclaimAngle::Removed => "but it looks like the link has since been removed",
        ReclaimAngle::Nofollowed => "but it's now marked nofollow",
        ReclaimAngle::SourcePageError => "but the page it was on seems to be down or to have moved",
    };
    let ask = match pitch.angle {
        ReclaimAngle::Removed => {
            "Would you be open to restoring it? It's still a relevant, \
useful resource for your readers, and I'm happy to confirm anything you need."
        }
        ReclaimAngle::Nofollowed => {
            "Would you consider switching it back to a standard (dofollow) link?"
        }
        ReclaimAngle::SourcePageError => {
            "If the page has moved, is there a new home where that link could live?"
        }
    };

    let context =
        format!("I noticed {source} linked to {target}{anchor_part}{when_part}, {lost_phrase}.");
    let body = format!("{greeting}\n\n{context} {ask}\n\n{sign_off}");
    OutreachDraft { subject, body }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx() -> OutreachContext<'static> {
        OutreachContext {
            prospect_domain: "example.com",
            contact_name: None,
            target_url: None,
            sender_name: None,
            campaign_name: "Q4 Link Building",
            reclaim: None,
        }
    }

    #[test]
    fn subject_uses_domain_when_present() {
        assert_eq!(
            render_draft(ctx()).subject,
            "Quick question about example.com"
        );
    }

    #[test]
    fn subject_falls_back_to_campaign_without_domain() {
        let mut c = ctx();
        c.prospect_domain = "";
        assert_eq!(render_draft(c).subject, "Collaboration — Q4 Link Building");
    }

    #[test]
    fn greeting_uses_contact_name_or_there() {
        assert!(render_draft(ctx()).body.starts_with("Hi there,"));
        let mut c = ctx();
        c.contact_name = Some("Dana");
        assert!(render_draft(c).body.starts_with("Hi Dana,"));
    }

    #[test]
    fn body_mentions_target_url_when_given() {
        let mut c = ctx();
        c.target_url = Some("https://ours.com/guide");
        assert!(render_draft(c).body.contains("https://ours.com/guide"));
    }

    #[test]
    fn sign_off_includes_sender_when_given() {
        let mut c = ctx();
        c.sender_name = Some("Alex");
        assert!(render_draft(c).body.trim_end().ends_with("Alex"));
    }

    #[test]
    fn deterministic() {
        let mut c = ctx();
        c.contact_name = Some("Dana");
        c.target_url = Some("https://ours.com/x");
        c.sender_name = Some("Alex");
        assert_eq!(render_draft(c), render_draft(c));
    }

    #[test]
    fn reclaim_removed_pitch_leads_with_receipts() {
        let mut c = ctx();
        c.target_url = Some("https://me.com/x");
        c.reclaim = Some(ReclaimPitch {
            angle: ReclaimAngle::Removed,
            source_url: "https://blog.example/post",
            anchor: Some("best tools"),
            lost_on: Some("2026-10-01"),
        });
        let d = render_draft(c);
        assert_eq!(d.subject, "Your link to us on example.com");
        assert!(d.body.contains("https://blog.example/post"));
        assert!(d.body.contains("https://me.com/x"));
        assert!(d.body.contains("(anchor \"best tools\")"));
        assert!(d.body.contains("around 2026-10-01"));
        assert!(d.body.contains("has since been removed"));
        assert!(d.body.contains("restoring it"));
    }

    #[test]
    fn reclaim_nofollow_and_source_error_asks_differ() {
        let mut c = ctx();
        c.reclaim = Some(ReclaimPitch {
            angle: ReclaimAngle::Nofollowed,
            source_url: "https://blog.example/post",
            anchor: None,
            lost_on: None,
        });
        assert!(render_draft(c).body.contains("dofollow"));

        c.reclaim = Some(ReclaimPitch {
            angle: ReclaimAngle::SourcePageError,
            source_url: "https://blog.example/post",
            anchor: None,
            lost_on: None,
        });
        let d = render_draft(c);
        assert!(d.body.contains("moved"));
        // Missing optionals render cleanly — no dangling anchor/date fragments.
        assert!(!d.body.contains("(anchor"));
        assert!(!d.body.contains("around"));
    }
}
