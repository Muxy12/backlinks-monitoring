//! Scheduled Google Search Console ingestion (Post-Link Impact). Once a day per
//! connected property, pull page-level Search Analytics (dimensions [date, page])
//! over a trailing window and upsert into `gsc_daily`. **One request per
//! property** (not per backlink), so the Impact service later reads `gsc_daily`
//! locally with zero external calls. Runs inside the internal tick, bounded.

use crate::routes::gsc_routes::{OAuthEnv, oauth_env, token_key};
use backlio_core::{NormalizeOptions, normalize};
use db::repositories::{gsc, gsc_daily};
use db::sqlx::PgPool;
use std::time::Duration;

const TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
const SITES_BASE: &str = "https://www.googleapis.com/webmasters/v3/sites/";
const LOOKBACK_DAYS: i64 = 90;
const GSC_DELAY_DAYS: i64 = 2; // Search Console data lags ~2–3 days.
const STALE_HOURS: i64 = 20;
const FORM_CT: &str = "application/x-www-form-urlencoded";

fn urlencode(pairs: &[(&str, &str)]) -> String {
    let mut u = reqwest::Url::parse("http://x/").expect("static base url");
    {
        let mut q = u.query_pairs_mut();
        for (k, v) in pairs {
            q.append_pair(k, v);
        }
    }
    u.query().unwrap_or("").to_string()
}

/// Percent-encode a GSC property for a single path segment — encodes EVERYTHING
/// except RFC-3986 unreserved chars, so `sc-domain:x` → `sc-domain%3Ax` and
/// `https://x/` → `https%3A%2F%2Fx%2F` (audit #2: no path injection via site_url).
fn pct_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

fn sa_url(site_url: &str) -> Option<reqwest::Url> {
    reqwest::Url::parse(&format!(
        "{SITES_BASE}{}/searchAnalytics/query",
        pct_encode(site_url)
    ))
    .ok()
}

async fn refresh_access_token(
    client: &reqwest::Client,
    env: &OAuthEnv,
    refresh: &str,
) -> Option<String> {
    let form = [
        ("refresh_token", refresh),
        ("client_id", env.client_id.as_str()),
        ("client_secret", env.client_secret.as_str()),
        ("grant_type", "refresh_token"),
    ];
    let v: serde_json::Value = client
        .post(TOKEN_URL)
        .header(reqwest::header::CONTENT_TYPE, FORM_CT)
        .body(urlencode(&form))
        .send()
        .await
        .ok()?
        .json()
        .await
        .ok()?;
    v.get("access_token")?.as_str().map(str::to_string)
}

async fn sync_one(
    client: &reqwest::Client,
    env: &OAuthEnv,
    key: &[u8; 32],
    pool: &PgPool,
    conn: &gsc::DueConnection,
) -> Result<u64, String> {
    let refresh = auth::decrypt_secret(key, &conn.refresh_token_enc).map_err(|_| "decrypt")?;
    let refresh = String::from_utf8(refresh).map_err(|_| "utf8")?;
    let access = refresh_access_token(client, env, &refresh)
        .await
        .ok_or("token refresh failed")?;
    let url = sa_url(&conn.site_url).ok_or("bad site_url")?;

    let today = chrono::Utc::now().date_naive();
    let end = today - chrono::Duration::days(GSC_DELAY_DAYS);
    let start = today - chrono::Duration::days(LOOKBACK_DAYS);
    let body = serde_json::json!({
        "startDate": start.to_string(),
        "endDate": end.to_string(),
        "dimensions": ["date", "page"],
        "rowLimit": 25000
    });
    let resp: serde_json::Value = client
        .post(url)
        .bearer_auth(access)
        .json(&body)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())?;

    let rows = resp
        .get("rows")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let opts = NormalizeOptions::default();
    let mut pages = Vec::new();
    let mut days = Vec::new();
    let mut imps = Vec::new();
    let mut clicks = Vec::new();
    let mut ctrs = Vec::new();
    let mut poss = Vec::new();
    for row in rows {
        let Some(keys) = row.get("keys").and_then(|k| k.as_array()) else {
            continue;
        };
        let (Some(day_s), Some(page_s)) = (
            keys.first().and_then(|v| v.as_str()),
            keys.get(1).and_then(|v| v.as_str()),
        ) else {
            continue;
        };
        let Ok(day) = chrono::NaiveDate::parse_from_str(day_s, "%Y-%m-%d") else {
            continue;
        };
        let page = normalize(page_s, None, opts).unwrap_or_else(|| page_s.to_string());
        pages.push(page);
        days.push(day);
        imps.push(
            row.get("impressions")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0)
                .round() as i32,
        );
        clicks.push(
            row.get("clicks")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0)
                .round() as i32,
        );
        ctrs.push(row.get("ctr").and_then(|v| v.as_f64()).unwrap_or(0.0));
        poss.push(row.get("position").and_then(|v| v.as_f64()).unwrap_or(0.0));
    }
    if pages.is_empty() {
        return Ok(0);
    }
    gsc_daily::upsert_many(
        pool,
        conn.organization_id,
        conn.project_id,
        &pages,
        &days,
        &imps,
        &clicks,
        &ctrs,
        &poss,
    )
    .await
    .map_err(|e| e.to_string())
}

/// Sync up to `limit` due properties. Returns how many synced OK. DB-only when
/// GSC is unconfigured (returns 0). Called from the internal tick.
pub async fn sync_due(pool: &PgPool, limit: i64) -> usize {
    let (Some(env), Some(key)) = (oauth_env(), token_key()) else {
        return 0;
    };
    let stale = chrono::Utc::now() - chrono::Duration::hours(STALE_HOURS);
    let due = match gsc::list_due(pool, stale, limit).await {
        Ok(d) => d,
        Err(e) => {
            tracing::warn!(error = %e, "gsc sync: list_due failed");
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
    let mut synced = 0;
    for conn in due {
        match sync_one(&client, &env, &key, pool, &conn).await {
            Ok(rows) => {
                tracing::info!(project = %conn.project_id, rows, "gsc sync ok");
                let _ = gsc::mark_synced(pool, conn.id, None).await;
                synced += 1;
            }
            Err(e) => {
                tracing::warn!(error = %e, project = %conn.project_id, "gsc sync failed");
                let _ = gsc::mark_synced(pool, conn.id, Some(&e)).await;
            }
        }
    }
    synced
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn site_url_is_fully_percent_encoded() {
        assert_eq!(
            pct_encode("sc-domain:example.com"),
            "sc-domain%3Aexample.com"
        );
        assert_eq!(
            pct_encode("https://example.com/"),
            "https%3A%2F%2Fexample.com%2F"
        );
    }

    #[test]
    fn sa_url_builds_safely() {
        let u = sa_url("sc-domain:example.com").unwrap();
        assert_eq!(
            u.as_str(),
            "https://www.googleapis.com/webmasters/v3/sites/sc-domain%3Aexample.com/searchAnalytics/query"
        );
        // A malicious value can't escape the segment.
        let u = sa_url("../../evil").unwrap();
        assert!(u.as_str().contains("%2E%2E%2F%2E%2E%2Fevil"));
    }
}
