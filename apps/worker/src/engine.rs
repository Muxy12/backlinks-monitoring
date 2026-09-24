//! The check engine: fetch a backlink's source, classify the result into an
//! [`Observation`], run the pure state machine, and persist the check + events +
//! new state atomically (docs/06, docs/07, docs/08). This is where the crawler,
//! the core domain, and the database meet — the orchestration is here so each of
//! those stays independently testable.

use backlio_core::{
    BacklinkState, ConnErrorKind, DetectedFields, NormalizeOptions, Observation, RelType,
    TransitionPolicy, transition,
};
use crawler::{CrawlerConfig, FetchError, FetchOutcome, fetch, parse_page};
use db::repositories::monitor::{self, CheckWrite, EventWrite, TransitionWrite};
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

/// Check one backlink end to end. Returns `true` when a check was applied,
/// `false` when it was a no-op (idempotent replay of the same crawl).
pub async fn process_backlink_check(
    pool: &PgPool,
    cfg: &CrawlerConfig,
    policy: &TransitionPolicy,
    backlink_id: Uuid,
) -> anyhow::Result<bool> {
    let st = monitor::load_state(pool, backlink_id).await?;
    let current = build_state(&st);
    let crawl_id = Uuid::now_v7();

    let outcome = fetch(cfg, &st.source_url).await;
    let (obs, source_status, final_url, duration_ms, bytes, error_kind) =
        classify(&outcome, &st.target_url);

    let now = chrono::Utc::now();
    let t = transition(&current, &obs, now, policy);

    let found = matches!(obs, Observation::Present(_));
    let check = CheckWrite {
        crawl_id,
        found,
        status_after: t.status.as_str().to_string(),
        source_status,
        anchor_text: t.detected.anchor_text.clone(),
        rel_attributes: Some(t.detected.rel_attributes.clone()),
        is_dofollow: t.detected.is_dofollow,
        final_url,
        duration_ms,
        bytes,
        error_kind,
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
        source_http_status: source_status,
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

    let applied =
        monitor::apply_check(pool, st.organization_id, backlink_id, check, tw, events).await?;
    Ok(applied)
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
        },
        consecutive_failure_count: st.consecutive_failure_count,
        last_seen_at: st.last_seen_at,
    }
}

type Classified = (
    Observation,
    Option<i32>,    // source_status
    Option<String>, // final_url
    Option<i32>,    // duration_ms
    Option<i32>,    // bytes
    Option<String>, // error_kind
);

/// Map a fetch outcome to an [`Observation`] plus the raw fields the check row
/// records. Followed redirects are transparent (we evaluate the final page);
/// `SourceRedirected` is reserved for a future explicit-redirect signal.
fn classify(outcome: &FetchOutcome, target_url: &str) -> Classified {
    let opts = NormalizeOptions::default();
    match outcome {
        FetchOutcome::Response(ok) => {
            let status = ok.status as i32;
            let dur = Some(ok.duration_ms as i32);
            let bytes = Some(ok.bytes as i32);
            let is_html = ok
                .content_type
                .as_deref()
                .map(|c| c.contains("html"))
                .unwrap_or(true); // no content-type: attempt a parse
            if (200..300).contains(&ok.status) && is_html {
                let mut page = parse_page(&ok.body);
                // Fold in a header-level X-Robots-Tag noindex (docs/06 §3.5).
                if ok.header_noindex {
                    page.robots_noindex = true;
                }
                let obs = crawler::evaluate(&page, target_url, &ok.final_url, opts);
                (
                    obs,
                    Some(status),
                    Some(ok.final_url.clone()),
                    dur,
                    bytes,
                    None,
                )
            } else if ok.status == 404 || ok.status == 410 {
                (
                    Observation::SourceGone { status: ok.status },
                    Some(status),
                    Some(ok.final_url.clone()),
                    dur,
                    bytes,
                    Some("gone".into()),
                )
            } else if ok.status >= 400 {
                (
                    Observation::SourceHttpError { status: ok.status },
                    Some(status),
                    Some(ok.final_url.clone()),
                    dur,
                    bytes,
                    Some("http_error".into()),
                )
            } else {
                // 2xx non-HTML: we cannot verify the link on this content.
                (
                    Observation::SourceBlocked {
                        reason: backlio_core::BlockReason::NonHtmlContent,
                    },
                    Some(status),
                    Some(ok.final_url.clone()),
                    dur,
                    bytes,
                    Some("non_html".into()),
                )
            }
        }
        FetchOutcome::Failed(err) => {
            let obs = match err {
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
                    reason: backlio_core::BlockReason::SsrfGuard,
                },
                FetchError::Oversized => Observation::SourceBlocked {
                    reason: backlio_core::BlockReason::OversizedBody,
                },
                FetchError::BadUrl => Observation::SourceBlocked {
                    reason: backlio_core::BlockReason::SsrfGuard,
                },
            };
            (obs, None, None, None, None, Some(format!("{err:?}")))
        }
    }
}
