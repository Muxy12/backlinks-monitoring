//! HTML parsing: extract links, `<base href>`, canonical, and robots
//! directives from a page (docs/06 §3, §3.3). Parse **once** per source page;
//! the matcher then evaluates every expected backlink against this one result
//! (§27, docs/06 §4).

use backlio_core::{RelSet, RelType};
use scraper::{ElementRef, Html, Selector};
use std::sync::OnceLock;

/// A single `<a href>` found on the page, before normalization/matching.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtractedLink {
    /// The raw `href` attribute, unresolved.
    pub href_raw: String,
    /// Anchor text, whitespace-collapsed. `None` when the anchor has no text
    /// content at all; an image-only anchor is rendered as `[img: <alt>]`
    /// (docs/06 §3.2) rather than dropped.
    pub anchor_text: Option<String>,
    /// The link's `rel` token set.
    pub rel: RelSet,
    /// `true` when the anchor wraps only an image (no text) — an image link.
    pub is_image: bool,
    /// The image's `alt` when `is_image` (may be empty); `None` for text links.
    pub image_alt: Option<String>,
    /// Best-effort page region the link sits in (docs/06): content, header,
    /// footer, sidebar, navigation, author_bio, comment, or unknown.
    pub placement: String,
    /// A short snippet of the text around the link, for context (may be None).
    pub surrounding_text: Option<String>,
    /// Document order (0-based) — used to break ties between multiple matches.
    pub position: usize,
}

/// Classify where a link sits on the page by walking its ancestors from nearest
/// to farthest and taking the first recognizable region. Heuristic and
/// deliberately conservative — unknown is fine, a wrong guess is not.
fn detect_placement(a: ElementRef) -> String {
    for anc in a.ancestors() {
        let Some(el) = anc.value().as_element() else {
            continue;
        };
        let tag = el.name();
        let idc = format!(
            "{} {}",
            el.attr("id").unwrap_or(""),
            el.attr("class").unwrap_or("")
        )
        .to_ascii_lowercase();
        let role = el.attr("role").unwrap_or("").to_ascii_lowercase();
        let has = |kw: &str| idc.contains(kw);

        if has("comment") {
            return "comment".into();
        }
        if has("author") || has("byline") || has("bio") {
            return "author_bio".into();
        }
        if tag == "footer" || role == "contentinfo" || has("footer") {
            return "footer".into();
        }
        if tag == "header" || role == "banner" || has("masthead") {
            return "header".into();
        }
        if tag == "nav" || role == "navigation" || has("menu") || has("navbar") || has("nav") {
            return "navigation".into();
        }
        if tag == "aside" || role == "complementary" || has("sidebar") || has("widget") {
            return "sidebar".into();
        }
        if tag == "main" || tag == "article" || has("content") || has("post") || has("entry") {
            return "content".into();
        }
    }
    "unknown".into()
}

/// A short, whitespace-collapsed snippet from the link's nearest block ancestor,
/// for showing the link in context. Truncated so it never bloats a row.
fn surrounding_text(a: ElementRef) -> Option<String> {
    let parent = a.parent().and_then(ElementRef::wrap)?;
    let text: String = parent
        .text()
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    if text.is_empty() {
        return None;
    }
    let snippet: String = text.chars().take(280).collect();
    Some(snippet)
}

/// Everything the matcher needs from one parsed page.
#[derive(Debug, Clone)]
pub struct ParsedPage {
    /// `<base href>` if present — relative links resolve against this, else the
    /// page's final URL (docs/06 §3.1).
    pub base_href: Option<String>,
    /// `<link rel="canonical" href>` if present.
    pub canonical: Option<String>,
    /// Page-level `<meta name="robots" content="noindex">` (or `X-Robots-Tag`,
    /// supplied by the fetch layer and OR'd in there).
    pub robots_noindex: bool,
    /// Page-level `nofollow` — makes **every** link on the page nofollow,
    /// regardless of per-link `rel` (docs/06 §3.3).
    pub robots_nofollow: bool,
    pub links: Vec<ExtractedLink>,
}

fn sel(cache: &'static OnceLock<Selector>, s: &str) -> &'static Selector {
    cache.get_or_init(|| Selector::parse(s).expect("static selector is valid"))
}

/// Parse a page's HTML. Pure: same bytes in, same result out.
pub fn parse_page(html: &str) -> ParsedPage {
    static A: OnceLock<Selector> = OnceLock::new();
    static BASE: OnceLock<Selector> = OnceLock::new();
    static CANON: OnceLock<Selector> = OnceLock::new();
    static ROBOTS: OnceLock<Selector> = OnceLock::new();
    static IMG: OnceLock<Selector> = OnceLock::new();

    let doc = Html::parse_document(html);

    let base_href = doc
        .select(sel(&BASE, "base[href]"))
        .next()
        .and_then(|e| e.value().attr("href"))
        .map(str::to_string);

    let canonical = doc
        .select(sel(&CANON, "link[rel~=\"canonical\"][href]"))
        .next()
        .and_then(|e| e.value().attr("href"))
        .map(str::to_string);

    // Robots directives from any <meta name="robots"> (name match is
    // case-insensitive per the CSS `i` flag).
    let (mut robots_noindex, mut robots_nofollow) = (false, false);
    for meta in doc.select(sel(&ROBOTS, "meta[name][content]")) {
        let name = meta.value().attr("name").unwrap_or("");
        if name.eq_ignore_ascii_case("robots") {
            let content = meta
                .value()
                .attr("content")
                .unwrap_or("")
                .to_ascii_lowercase();
            for token in content.split(',') {
                match token.trim() {
                    "noindex" => robots_noindex = true,
                    "nofollow" => robots_nofollow = true,
                    "none" => {
                        robots_noindex = true;
                        robots_nofollow = true;
                    }
                    _ => {}
                }
            }
        }
    }

    let img_sel = sel(&IMG, "img");
    let mut links = Vec::new();
    for (position, a) in doc.select(sel(&A, "a[href]")).enumerate() {
        let href_raw = match a.value().attr("href") {
            Some(h) if !h.trim().is_empty() => h.to_string(),
            _ => continue, // empty href is not a link we track
        };

        // Anchor text: collapse whitespace. An anchor with no text but an
        // <img> is an *image link*: record its nature + alt, and still render
        // a `[img: <alt>]` marker for the text comparison (docs/06 §3.2).
        let text: String = a
            .text()
            .collect::<String>()
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        let img = a.select(img_sel).next();
        let is_image = text.is_empty() && img.is_some();
        let image_alt: Option<String> = if is_image {
            Some(
                img.and_then(|i| i.value().attr("alt"))
                    .unwrap_or("")
                    .trim()
                    .to_string(),
            )
        } else {
            None
        };
        let anchor_text = if !text.is_empty() {
            Some(text)
        } else if is_image {
            Some(format!("[img: {}]", image_alt.as_deref().unwrap_or("")))
        } else {
            None
        };

        let rel = parse_rel(a.value().attr("rel").unwrap_or(""));
        let placement = detect_placement(a);
        let surrounding = surrounding_text(a);

        links.push(ExtractedLink {
            href_raw,
            anchor_text,
            rel,
            is_image,
            image_alt,
            placement,
            surrounding_text: surrounding,
            position,
        });
    }

    ParsedPage {
        base_href,
        canonical,
        robots_noindex,
        robots_nofollow,
        links,
    }
}

/// Parse a `rel` attribute value into the tokens we model (docs/06 §3.3).
/// Unknown tokens (`noopener`, `me`, …) are ignored — they do not affect
/// follow semantics.
pub fn parse_rel(value: &str) -> RelSet {
    let mut set = RelSet::new();
    for token in value.split_whitespace() {
        match token.to_ascii_lowercase().as_str() {
            "nofollow" => set.insert(RelType::Nofollow),
            "sponsored" => set.insert(RelType::Sponsored),
            "ugc" => set.insert(RelType::Ugc),
            "dofollow" => set.insert(RelType::Dofollow),
            _ => {}
        }
    }
    set
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_a_simple_dofollow_link() {
        let page = parse_page(
            r#"<html><body><a href="https://example.com/x">SEO tools</a></body></html>"#,
        );
        assert_eq!(page.links.len(), 1);
        let l = &page.links[0];
        assert_eq!(l.href_raw, "https://example.com/x");
        assert_eq!(l.anchor_text.as_deref(), Some("SEO tools"));
        assert!(l.rel.is_empty());
        assert!(l.rel.is_dofollow(false));
    }

    #[test]
    fn parses_rel_nofollow_sponsored_ugc() {
        let page = parse_page(
            r#"<a href="/a" rel="nofollow">a</a>
               <a href="/b" rel="sponsored noopener">b</a>
               <a href="/c" rel="ugc">c</a>"#,
        );
        assert!(page.links[0].rel.contains(RelType::Nofollow));
        assert!(!page.links[0].rel.is_dofollow(false));
        assert!(page.links[1].rel.contains(RelType::Sponsored));
        assert!(page.links[2].rel.contains(RelType::Ugc));
    }

    #[test]
    fn whitespace_in_anchor_is_collapsed() {
        let page = parse_page("<a href=\"/x\">  SEO\n  tools  </a>");
        assert_eq!(page.links[0].anchor_text.as_deref(), Some("SEO tools"));
    }

    #[test]
    fn image_only_anchor_becomes_img_marker() {
        let page = parse_page(r#"<a href="/x"><img src="logo.png" alt="Acme logo"></a>"#);
        assert_eq!(
            page.links[0].anchor_text.as_deref(),
            Some("[img: Acme logo]")
        );
    }

    #[test]
    fn image_only_anchor_without_alt() {
        let page = parse_page(r#"<a href="/x"><img src="logo.png"></a>"#);
        assert_eq!(page.links[0].anchor_text.as_deref(), Some("[img: ]"));
    }

    #[test]
    fn base_href_is_extracted() {
        let page = parse_page(
            r#"<head><base href="https://example.com/blog/"></head><body><a href="post">p</a></body>"#,
        );
        assert_eq!(page.base_href.as_deref(), Some("https://example.com/blog/"));
    }

    #[test]
    fn canonical_is_extracted() {
        let page = parse_page(
            r#"<head><link rel="canonical" href="https://example.com/canonical"></head>"#,
        );
        assert_eq!(
            page.canonical.as_deref(),
            Some("https://example.com/canonical")
        );
    }

    #[test]
    fn meta_robots_noindex_nofollow() {
        let page = parse_page(r#"<head><meta name="robots" content="noindex, nofollow"></head>"#);
        assert!(page.robots_noindex);
        assert!(page.robots_nofollow);
    }

    #[test]
    fn meta_robots_none_means_both() {
        let page = parse_page(r#"<meta name="ROBOTS" content="none">"#);
        assert!(page.robots_noindex && page.robots_nofollow);
    }

    #[test]
    fn page_nofollow_overrides_per_link_dofollow() {
        let page = parse_page(
            r#"<head><meta name="robots" content="nofollow"></head><body><a href="/x">x</a></body>"#,
        );
        // The link has no rel, but the page is nofollow.
        assert!(!page.links[0].rel.is_dofollow(page.robots_nofollow));
    }

    #[test]
    fn empty_href_is_skipped() {
        let page =
            parse_page(r#"<a href="">empty</a><a href="  ">blank</a><a href="/real">real</a>"#);
        assert_eq!(page.links.len(), 1);
        assert_eq!(page.links[0].href_raw, "/real");
    }

    #[test]
    fn malformed_html_still_parses_what_it_can() {
        // Unclosed tags, stray markup — html5ever is lenient (docs/06 §7).
        let page = parse_page(r#"<html><body><a href="/x">broken <b>bold</a> <a href="/y">two"#);
        assert!(page.links.len() >= 2);
        assert_eq!(page.links[0].href_raw, "/x");
    }

    #[test]
    fn position_reflects_document_order() {
        let page = parse_page(r#"<a href="/first">1</a><a href="/second">2</a>"#);
        assert_eq!(page.links[0].position, 0);
        assert_eq!(page.links[1].position, 1);
    }
}
