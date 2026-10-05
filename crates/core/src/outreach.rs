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
}

/// A rendered, ready-to-copy draft.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutreachDraft {
    pub subject: String,
    pub body: String,
}

/// Render a first-contact outreach draft. Deterministic.
pub fn render_draft(ctx: OutreachContext<'_>) -> OutreachDraft {
    let domain = ctx.prospect_domain.trim();
    let greeting = match ctx.contact_name.map(str::trim).filter(|s| !s.is_empty()) {
        Some(name) => format!("Hi {name},"),
        None => "Hi there,".to_string(),
    };
    let subject = if domain.is_empty() {
        format!("Collaboration — {}", ctx.campaign_name.trim())
    } else {
        format!("Quick question about {domain}")
    };

    // The ask: reference our target page when we have one, otherwise stay generic.
    let ask = match ctx.target_url.map(str::trim).filter(|s| !s.is_empty()) {
        Some(url) => format!(
            "I've been following {site} and think our resource at {url} would be a \
genuinely useful addition for your readers. Would you be open to including a link \
to it where it fits naturally?",
            site = if domain.is_empty() {
                "your site"
            } else {
                domain
            },
            url = url,
        ),
        None => format!(
            "I've been following {site} and would love to explore a collaboration — \
I think there's a natural fit between what you publish and the resources we're \
building. Could I share a couple of ideas?",
            site = if domain.is_empty() {
                "your site"
            } else {
                domain
            },
        ),
    };

    let sign_off = match ctx.sender_name.map(str::trim).filter(|s| !s.is_empty()) {
        Some(name) => format!("Thanks for your time,\n{name}"),
        None => "Thanks for your time,".to_string(),
    };

    let body = format!("{greeting}\n\n{ask}\n\n{sign_off}");
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
}
