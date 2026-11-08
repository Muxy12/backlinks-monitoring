//! Refonte P2: build a project site's semantic profile. Crawls the site's own
//! pages (homepage + one click away, SSRF-safe) and stores a heuristic baseline
//! — language from `<html lang>`, salient terms from the title/meta/headings.
//! No external cost. The LLM upgrade (P2.3) plugs in right where the heuristic
//! runs, replacing it with a real semantic analysis when a model is configured.

use crawler::CrawlerConfig;
use uuid::Uuid;

/// Max pages one click from the homepage to crawl for the corpus.
const MAX_P1: usize = 4;
/// Max detected keywords to keep.
const MAX_KEYWORDS: usize = 12;

pub async fn run_analysis(
    pool: &sqlx::PgPool,
    crawler: &CrawlerConfig,
    org: Uuid,
    project: Uuid,
) -> anyhow::Result<()> {
    use db::repositories::{projects, website_analyses};

    website_analyses::mark_running(pool, project).await?;

    let domain = projects::domain_by_id(pool, org, project)
        .await?
        .unwrap_or_default();
    if domain.trim().is_empty() {
        website_analyses::set_error(pool, project, "project has no domain to analyse").await?;
        return Ok(());
    }

    let url = format!("https://{}", domain.trim());
    let scan = crawler::scan_site(crawler, &url, MAX_P1).await;

    // Corpus for the heuristic: the homepage title, meta description and every
    // heading collected across the crawl.
    let mut texts: Vec<String> = Vec::new();
    if let Some(t) = &scan.title {
        texts.push(t.clone());
    }
    if let Some(d) = &scan.meta_description {
        texts.push(d.clone());
    }
    texts.extend(scan.h1s.iter().map(|h| h.text.clone()));

    if texts.is_empty() {
        website_analyses::set_error(pool, project, "could not read any content from the site")
            .await?;
        return Ok(());
    }

    let keywords = backlio_core::salient_terms(&texts, MAX_KEYWORDS);
    let theme = keywords.first().cloned();
    let summary = format!(
        "Heuristic profile from {} page(s). Configure an LLM for a full semantic analysis \
         (sub-themes, products, summary).",
        scan.pages_scanned
    );

    website_analyses::set_result(
        pool,
        project,
        "heuristic",
        scan.lang.as_deref(),
        theme.as_deref(),
        &[],
        &[],
        &keywords,
        Some(&summary),
    )
    .await?;
    Ok(())
}
