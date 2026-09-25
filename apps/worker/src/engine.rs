//! The check engine: fetch a source page, classify the result into an
//! [`Observation`] per backlink, run the pure state machine, and persist the
//! check + events + new state atomically (docs/06, docs/07, docs/08).
//!
//! The unit of work is a **source group** (§27, docs/06 §4): one HTTP fetch and
//! one parse serve every backlink that shares that source page. A single-backlink
//! "check now" is just a group of one.

use backlio_core::{
    BacklinkState, BlockReason, ConnErrorKind, DetectedFields, NormalizeOptions, Observation,
    RelType, TransitionPolicy, transition,
};
use crawler::{CrawlerConfig, FetchError, FetchOutcome, ParsedPage, fetch, parse_page};
use db::repositories::monitor::{self, CheckWrite, EventWrite, TransitionWrite};
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

/// The fetch metadata shared by every backlink in a source group.
struct FetchMeta {
    source_status: Option<i32>,
    final_url: Option<String>,
    duration_ms: Option<i32>,
    bytes: Option<i32>,
    error_kind: Option<String>,
}

/// Check every backlink that shares one source page: fetch + parse **once**,
/// then evaluate each backlink's target against that single result (§27).
/// Returns how many checks were applied.
pub async fn check_source_group(
    pool: &PgPool,
    cfg: &CrawlerConfig,
    policy: &TransitionPolicy,
    source_url: &str,
    backlink_ids: &[Uuid],
) -> anyhow::Result<usize> {
    let outcome = fetch(cfg, source_url).await;
    let crawl_id = Uuid::now_v7(); // shared: ties the group's checks together
    let meta = fetch_meta(&outcome);

    // Parse once when we have an HTML 2xx; otherwise all backlinks share the
    // same target-independent failure observation.
    let parsed: Option<(ParsedPage, String)> = match &outcome {
        FetchOutcome::Response(ok) if (200..300).contains(&ok.status) && is_html(ok) => {
            let mut page = parse_page(&ok.body);
            if ok.header_noindex {
                page.robots_noindex = true;
            }
            Some((page, ok.final_url.clone()))
        }
        _ => None,
    };
    let shared_failure = failure_observation(&outcome);

    let opts = NormalizeOptions::default();
    let now = chrono::Utc::now();
    let mut applied = 0usize;
    // Dedup target fetches within this source group: several backlinks can point
    // at the same target. Maps a target URL -> (final HTTP status, final URL).
    let mut target_cache: std::collections::HashMap<String, (Option<i32>, Option<String>)> =
        std::collections::HashMap::new();

    for &id in backlink_ids {
        let st = monitor::load_state(pool, id).await?;
        let current = build_state(&st);
        let obs = match &parsed {
            Some((page, final_url)) => crawler::evaluate(page, &st.target_url, final_url, opts),
            None => shared_failure.clone(),
        };
        let t = transition(&current, &obs, now, policy);
        let found = matches!(obs, Observation::Present(_));

        // Only when the link is present do we spend a request resolving where the
        // target itself lands (after its own redirects). Reuses the SSRF-safe
        // `fetch`, so a target pointing at a private address is blocked. Failures
        // yield None and never overwrite a previously-known value (COALESCE in
        // the UPDATE), so "we could not check the target" is not "target gone".
        let (target_status, target_final) = if found && cfg.check_target {
            if let Some(hit) = target_cache.get(&st.target_url) {
                hit.clone()
            } else {
                let r = match fetch(cfg, &st.target_url).await {
                    FetchOutcome::Response(ok) => (Some(ok.status as i32), Some(ok.final_url)),
                    FetchOutcome::Failed(_) => (None, None),
                };
                target_cache.insert(st.target_url.clone(), r.clone());
                r
            }
        } else {
            (None, None)
        };

        let check = CheckWrite {
            crawl_id,
            found,
            status_after: t.status.as_str().to_string(),
            source_status: meta.source_status,
            target_status,
            anchor_text: t.detected.anchor_text.clone(),
            rel_attributes: Some(t.detected.rel_attributes.clone()),
            is_dofollow: t.detected.is_dofollow,
            final_url: meta.final_url.clone(),
            duration_ms: meta.duration_ms,
            bytes: meta.bytes,
            error_kind: if found { None } else { meta.error_kind.clone() },
        };
        let tw = TransitionWrite {
            status: t.status.as_str().to_string(),
            previous_status: t.previous_status.as_str().to_string(),
            consecutive_failure_count: t.consecutive_failure_count,
            next_check_at: t.next_check_at,
            last_seen_at: t.last_seen_at,
            detected_anchor_text: t.detected.anchor_text.clone(),
            detected_target_url: t.detected.target_url.clone(),
            detected_rel_attributes: Some(t.detected.rel_attributes.clone()),
            detected_is_dofollow: t.detected.is_dofollow,
            source_canonical: t.detected.canonical.clone(),
            source_robots_noindex: t.detected.noindex,
            source_http_status: meta.source_status,
            // Link nature only when we actually detected a link this cycle.
            link_type: t
                .detected
                .target_url
                .as_ref()
                .map(|_| if t.detected.is_image { "image" } else { "text" }.to_string()),
            image_alt: t.detected.image_alt.clone(),
            target_http_status: target_status,
            target_final_url: target_final,
        };
        let events: Vec<EventWrite> = t
            .events
            .iter()
            .map(|e| EventWrite {
                event_type: e.event_type.as_str().to_string(),
                previous_data: e.previous.clone().map(|v| json!(v)),
                current_data: e.current.clone().map(|v| json!(v)),
            })
            .collect();

        if monitor::apply_check(pool, st.organization_id, id, check, tw, events).await? {
            applied += 1;
        }
    }
    Ok(applied)
}

/// Check a single backlink (the "check now" path, §36) — a source group of one.
pub async fn process_backlink_check(
    pool: &PgPool,
    cfg: &CrawlerConfig,
    policy: &TransitionPolicy,
    backlink_id: Uuid,
) -> anyhow::Result<bool> {
    let st = monitor::load_state(pool, backlink_id).await?;
    let n = check_source_group(pool, cfg, policy, &st.source_url, &[backlink_id]).await?;
    Ok(n > 0)
}

fn is_html(ok: &crawler::FetchOk) -> bool {
    ok.content_type
        .as_deref()
        .map(|c| c.contains("html"))
        .unwrap_or(true) // no content-type: attempt a parse
}

fn fetch_meta(outcome: &FetchOutcome) -> FetchMeta {
    match outcome {
        FetchOutcome::Response(ok) => FetchMeta {
            source_status: Some(ok.status as i32),
            final_url: Some(ok.final_url.clone()),
            duration_ms: Some(ok.duration_ms as i32),
            bytes: Some(ok.bytes as i32),
            error_kind: if (200..300).contains(&ok.status) {
                None
            } else {
                Some(format!("http_{}", ok.status))
            },
        },
        FetchOutcome::Failed(err) => FetchMeta {
            source_status: None,
            final_url: None,
            duration_ms: None,
            bytes: None,
            error_kind: Some(format!("{err:?}")),
        },
    }
}

/// The observation shared by every backlink when the fetch did not yield an
/// HTML 2xx (target-independent: it is about the source page, not the link).
fn failure_observation(outcome: &FetchOutcome) -> Observation {
    match outcome {
        FetchOutcome::Response(ok) => {
            if (200..300).contains(&ok.status) && is_html(ok) {
                // Parsed path handles this per-backlink; placeholder unused.
                Observation::Absent
            } else if ok.status == 404 || ok.status == 410 {
                Observation::SourceGone { status: ok.status }
            } else if ok.status >= 400 {
                Observation::SourceHttpError { status: ok.status }
            } else {
                Observation::SourceBlocked {
                    reason: BlockReason::NonHtmlContent,
                }
            }
        }
        FetchOutcome::Failed(err) => match err {
            FetchError::Timeout => Observation::SourceUnreachable {
                kind: ConnErrorKind::Timeout,
            },
            FetchError::TooManyRedirects => Observation::SourceUnreachable {
                kind: ConnErrorKind::TooManyRedirects,
            },
            FetchError::Connect(_) => Observation::SourceUnreachable {
                kind: ConnErrorKind::ConnectionRefused,
            },
            FetchError::Blocked(_) => Observation::SourceBlocked {
                reason: BlockReason::SsrfGuard,
            },
            FetchError::Oversized => Observation::SourceBlocked {
                reason: BlockReason::OversizedBody,
            },
            FetchError::BadUrl => Observation::SourceBlocked {
                reason: BlockReason::SsrfGuard,
            },
        },
    }
}

/// Build the state-machine input from the persisted row.
fn build_state(st: &db::entities::MonitorState) -> BacklinkState {
    BacklinkState {
        status: backlio_core::BacklinkStatus::from_db(&st.status)
            .unwrap_or(backlio_core::BacklinkStatus::Pending),
        expected_anchor_text: st.expected_anchor_text.clone(),
        expected_rel_type: RelType::from_db(&st.expected_rel_type).unwrap_or(RelType::Dofollow),
        detected: DetectedFields {
            anchor_text: st.detected_anchor_text.clone(),
            target_url: st.detected_target_url.clone(),
            rel_attributes: st.detected_rel_attributes.clone().unwrap_or_default(),
            is_dofollow: st.detected_is_dofollow,
            canonical: st.source_canonical.clone(),
            noindex: st.source_robots_noindex,
            // Not carried on MonitorState — refreshed from each fresh detection.
            is_image: false,
            image_alt: None,
        },
        consecutive_failure_count: st.consecutive_failure_count,
        last_seen_at: st.last_seen_at,
    }
}

/// The daily warranty-expiry sweep (§32, docs/07 §5). Finds every warranted
/// link whose warranty lapses on or before `today + within_days` and records a
/// `warranty_expiring` notification for it, idempotently — a re-run on the same
/// day writes nothing new (the notification dedupe key is per backlink +
/// expiration date). Returns the number of *new* alerts written. This is a
/// system sweep across all orgs; each notification is still written scoped to
/// the backlink's own organization.
pub async fn run_warranty_sweep(
    pool: &PgPool,
    today: chrono::NaiveDate,
    within_days: i64,
    batch: i64,
) -> anyhow::Result<usize> {
    let cutoff = today + chrono::Duration::days(within_days.max(0));
    let due = db::repositories::warranties::expiring_before(pool, cutoff, batch).await?;
    let mut new_alerts = 0usize;
    for w in due {
        let lapsed = w.warranty_expiration <= today;
        let title = if lapsed {
            "Backlink warranty expired"
        } else {
            "Backlink warranty expiring soon"
        };
        let body = format!(
            "The warranty for a monitored backlink {} on {}.",
            if lapsed { "expired" } else { "expires" },
            w.warranty_expiration
        );
        if db::repositories::notifications::create_warranty_alert(
            pool,
            w.organization_id,
            w.backlink_id,
            w.warranty_expiration,
            title,
            &body,
        )
        .await?
        {
            new_alerts += 1;
        }
    }
    Ok(new_alerts)
}
