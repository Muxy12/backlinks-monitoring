//! Post-Link Impact — Phase 2: Google Search Console **URL Inspection** sweep.
//! Populates the Crawl + Indexation layers that were "unavailable" in the BETA.
//!
//! Quota-aware by design: the URL Inspection API is capped at ~2000 queries/day
//! per property, so we inspect ONLY the pages of tracked backlinks, re-inspect at
//! most every few days, and process a tiny bounded slice per tick. Reuses the
//! same OAuth env + token refresh as `gsc_sync`. Observation, never causation.

use crate::gsc_sync::refresh_access_token;
use crate::routes::gsc_routes::{oauth_env, token_key};
use db::repositories::gsc_inspection;
use db::sqlx::PgPool;
use std::time::Duration;

const INSPECT_URL: &str = "https://searchconsole.googleapis.com/v1/urlInspection/index:inspect";
/// Re-inspect a page at most this often (cadence: prudent).
const STALE_DAYS: i64 = 3;

fn rfc3339(v: &serde_json::Value) -> Option<chrono::DateTime<chrono::Utc>> {
    v.as_str()
        .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
        .map(|dt| dt.with_timezone(&chrono::Utc))
}

async fn inspect_one(
    client: &reqwest::Client,
    env: &crate::routes::gsc_routes::OAuthEnv,
    key: &[u8; 32],
    pool: &PgPool,
    due: &gsc_inspection::DueInspection,
) -> Result<(), String> {
    let refresh = auth::decrypt_secret(key, &due.refresh_token_enc).map_err(|_| "decrypt")?;
    let refresh = String::from_utf8(refresh).map_err(|_| "utf8")?;
    let access = refresh_access_token(client, env, &refresh)
        .await
        .ok_or("token refresh failed")?;

    let body = serde_json::json!({
        "inspectionUrl": due.page_url_normalized,
        "siteUrl": due.site_url,
    });
    let resp: serde_json::Value = client
        .post(INSPECT_URL)
        .bearer_auth(access)
        .json(&body)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())?;

    let idx = &resp["inspectionResult"]["indexStatusResult"];
    let s = |k: &str| idx.get(k).and_then(|v| v.as_str());
    let raw = resp.get("inspectionResult").cloned();

    gsc_inspection::upsert(
        pool,
        due.organization_id,
        due.project_id,
        &due.page_url_normalized,
        s("coverageState"),
        s("indexingState"),
        s("robotsTxtState"),
        s("pageFetchState"),
        s("verdict"),
        rfc3339(&idx["lastCrawlTime"]),
        s("googleCanonical"),
        s("userCanonical"),
        raw.as_ref(),
    )
    .await
    .map_err(|e| e.to_string())
}

/// Inspect up to `limit` due pages. Returns how many succeeded. Inert (returns 0)
/// when GSC isn't configured. Called from the internal tick, tightly bounded.
pub async fn inspect_due(pool: &PgPool, limit: i64) -> usize {
    let (Some(env), Some(key)) = (oauth_env(), token_key()) else {
        return 0;
    };
    let stale = chrono::Utc::now() - chrono::Duration::days(STALE_DAYS);
    let due = match gsc_inspection::list_due(pool, stale, limit).await {
        Ok(d) => d,
        Err(e) => {
            tracing::warn!(error = %e, "gsc inspect: list_due failed");
            return 0;
        }
    };
    if due.is_empty() {
        return 0;
    }
    let client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(20))
        .build()
    {
        Ok(c) => c,
        Err(_) => return 0,
    };
    let mut done = 0;
    for d in due {
        match inspect_one(&client, &env, &key, pool, &d).await {
            Ok(()) => {
                tracing::info!(project = %d.project_id, "gsc inspect ok");
                done += 1;
            }
            Err(e) => {
                tracing::warn!(error = %e, project = %d.project_id, "gsc inspect failed");
            }
        }
    }
    done
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rfc3339_parses_and_rejects() {
        let v = serde_json::json!("2026-09-01T12:30:00Z");
        assert!(rfc3339(&v).is_some());
        // Missing / wrong-typed / malformed → None (never panics).
        assert!(rfc3339(&serde_json::Value::Null).is_none());
        assert!(rfc3339(&serde_json::json!("not a date")).is_none());
        assert!(rfc3339(&serde_json::json!(12345)).is_none());
    }
}
