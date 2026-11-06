//! MCP server (docs/product/22 §3). JSON-RPC 2.0 over streamable HTTP at a
//! single `POST /mcp`, authenticated by an `lp_live_` Bearer key. Original
//! implementation — not derived from any third-party MCP code.
//!
//! Design rule: **read + analysis only** for the launch set. Each tool checks
//! the key's scope, delegates to the same repositories the cookie API uses, and
//! returns the data as a JSON text block. Provider keys never surface. Tool
//! availability follows the key's scopes.

use crate::routes::resource_routes::{BacklinkView, ClientView, ProjectView};
use crate::routes::spot_search_routes::{OfferView, SpotView};
use crate::state::AppState;
use axum::Json;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use backlio_core::{GeoSignals, SpotSignals, geo_score, spot_score};
use db::{ApiKeyContext, DbError, OrgScope, authorize_api_key};
use serde_json::{Value, json};
use uuid::Uuid;

const PROTOCOL_VERSION: &str = "2025-06-18";

/// The launch tool set: (name, required scope, one-line description).
const TOOLS: &[(&str, &str, &str)] = &[
    (
        "list_clients",
        "read:portfolio",
        "List the workspace's clients.",
    ),
    (
        "list_projects",
        "read:portfolio",
        "List projects (target sites).",
    ),
    (
        "portfolio_health",
        "read:portfolio",
        "Live vs at-risk monitored-link counts.",
    ),
    (
        "list_monitored_links",
        "read:portfolio",
        "Monitored links for a project (arg: project_id).",
    ),
    (
        "link_status",
        "read:portfolio",
        "One monitored link's status (arg: link_id).",
    ),
    (
        "recent_alerts",
        "read:portfolio",
        "Recent backlink change events.",
    ),
    (
        "usage",
        "read:portfolio",
        "Quota consumption and plan caps this period.",
    ),
    (
        "discover_spots",
        "read:discovery",
        "Ranked link spots for a project (arg: project_id).",
    ),
    (
        "explain_spot_score",
        "read:discovery",
        "A spot's explainable score breakdown (arg: spot_id).",
    ),
    (
        "geo_citations",
        "read:geo",
        "AI-cited domains for a project, scored + placeable (arg: project_id).",
    ),
    (
        "compare_offers",
        "read:marketplace",
        "Marketplace offers for a spot's domain (args: project_id, domain).",
    ),
];

fn tool_scope(name: &str) -> Option<&'static str> {
    TOOLS
        .iter()
        .find(|(n, _, _)| *n == name)
        .map(|(_, s, _)| *s)
}

/// `POST /mcp` — the single JSON-RPC endpoint.
pub async fn mcp(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Response {
    let ctx = match authenticate(&state, &headers).await {
        Ok(c) => c,
        Err(resp) => return resp,
    };

    let id = body.get("id").cloned().unwrap_or(Value::Null);
    let method = body.get("method").and_then(|m| m.as_str()).unwrap_or("");
    let params = body.get("params").cloned().unwrap_or(Value::Null);

    let response = match method {
        "initialize" => rpc_result(
            id,
            json!({
                "protocolVersion": PROTOCOL_VERSION,
                "capabilities": { "tools": {} },
                "serverInfo": { "name": "LinkPulz", "version": env!("CARGO_PKG_VERSION") },
            }),
        ),
        "notifications/initialized" => return StatusCode::ACCEPTED.into_response(),
        "tools/list" => rpc_result(id, json!({ "tools": tools_list(&ctx.scopes) })),
        "tools/call" => match call_tool(&state, &ctx, &params).await {
            Ok(value) => rpc_result(
                id,
                json!({
                    "content": [{ "type": "text", "text": to_pretty(&value) }],
                    "isError": false,
                }),
            ),
            Err(ToolError::Scope(scope)) => rpc_error(
                id,
                -32001,
                &format!("this key lacks the required scope: {scope}"),
            ),
            Err(ToolError::BadArgs(msg)) => rpc_error(id, -32602, &msg),
            Err(ToolError::NotFound) => rpc_error(id, -32004, "not found"),
            Err(ToolError::Db) => rpc_error(id, -32603, "internal error"),
        },
        other => rpc_error(id, -32601, &format!("method not found: {other}")),
    };

    Json(response).into_response()
}

// ── Auth ─────────────────────────────────────────────────────────────────────

async fn authenticate(state: &AppState, headers: &HeaderMap) -> Result<ApiKeyContext, Response> {
    let token = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .map(str::trim)
        .unwrap_or("");
    if !auth::looks_like_api_key(token) {
        return Err(unauthorized());
    }
    let hash = auth::hash_api_key(token);
    match authorize_api_key(&state.pool, &hash).await {
        Ok(ctx) => Ok(ctx),
        Err(DbError::Suspended) => Err((
            StatusCode::FORBIDDEN,
            Json(json!({ "error": "organization suspended" })),
        )
            .into_response()),
        _ => Err(unauthorized()),
    }
}

fn unauthorized() -> Response {
    (
        StatusCode::UNAUTHORIZED,
        Json(json!({ "error": "a valid lp_live_ API key is required" })),
    )
        .into_response()
}

// ── JSON-RPC envelopes ─────────────────────────────────────────────────────

fn rpc_result(id: Value, result: Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "result": result })
}

fn rpc_error(id: Value, code: i64, message: &str) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } })
}

fn to_pretty(v: &Value) -> String {
    serde_json::to_string_pretty(v).unwrap_or_else(|_| v.to_string())
}

fn tools_list(scopes: &[String]) -> Vec<Value> {
    TOOLS
        .iter()
        .filter(|(_, scope, _)| scopes.iter().any(|s| s == scope))
        .map(|(name, _, desc)| {
            json!({
                "name": name,
                "description": desc,
                "inputSchema": { "type": "object", "properties": {}, "additionalProperties": true },
            })
        })
        .collect()
}

// ── Tool dispatch ─────────────────────────────────────────────────────────

enum ToolError {
    Scope(&'static str),
    BadArgs(String),
    NotFound,
    Db,
}

impl From<DbError> for ToolError {
    fn from(e: DbError) -> Self {
        match e {
            DbError::NotFound => ToolError::NotFound,
            _ => ToolError::Db,
        }
    }
}

fn arg_uuid(args: &Value, key: &str) -> Result<Uuid, ToolError> {
    args.get(key)
        .and_then(|v| v.as_str())
        .and_then(|s| Uuid::parse_str(s).ok())
        .ok_or_else(|| ToolError::BadArgs(format!("missing or invalid `{key}` (expected a UUID)")))
}

async fn call_tool(
    state: &AppState,
    ctx: &ApiKeyContext,
    params: &Value,
) -> Result<Value, ToolError> {
    let name = params
        .get("name")
        .and_then(|n| n.as_str())
        .ok_or_else(|| ToolError::BadArgs("missing tool name".into()))?;
    let args = params.get("arguments").cloned().unwrap_or(Value::Null);

    // Every tool is gated by its declared scope.
    let needed = tool_scope(name).ok_or(ToolError::NotFound)?;
    if !ctx.scopes.iter().any(|s| s == needed) {
        return Err(ToolError::Scope(needed));
    }
    let scope: &OrgScope = &ctx.scope;
    let pool = &state.pool;
    use db::repositories as repo;

    match name {
        "list_clients" => {
            let rows = repo::clients::list(pool, scope).await?;
            Ok(json!(
                rows.into_iter().map(ClientView::from).collect::<Vec<_>>()
            ))
        }
        "list_projects" => {
            let rows = repo::projects::list(pool, scope).await?;
            Ok(json!(
                rows.into_iter().map(ProjectView::from).collect::<Vec<_>>()
            ))
        }
        "portfolio_health" => {
            let (live, at_risk) = repo::overview::link_health(pool, scope).await?;
            Ok(json!({ "live": live, "at_risk": at_risk }))
        }
        "list_monitored_links" => {
            let project_id = arg_uuid(&args, "project_id")?;
            let rows = repo::backlinks::list_for_project(pool, scope, project_id, 100).await?;
            Ok(json!(
                rows.into_iter().map(BacklinkView::from).collect::<Vec<_>>()
            ))
        }
        "link_status" => {
            let link_id = arg_uuid(&args, "link_id")?;
            let b = repo::backlinks::get(pool, scope, link_id).await?;
            Ok(json!(BacklinkView::from(b)))
        }
        "recent_alerts" => {
            let rows = repo::overview::recent_events(pool, scope, 30).await?;
            Ok(json!(
                rows.into_iter()
                    .map(|e| json!({
                        "event": e.event_type,
                        "detected_at": e.detected_at.to_rfc3339(),
                        "source_domain": e.source_domain,
                        "target_url": e.target_url,
                        "link_id": e.backlink_id,
                    }))
                    .collect::<Vec<_>>()
            ))
        }
        "usage" => {
            let spec = repo::billing::plan_for_org(pool, scope.organization_id())
                .await?
                .spec();
            let discoveries =
                repo::usage::discoveries_this_period(pool, scope.organization_id()).await?;
            let geo = repo::usage::geo_queries_this_period(pool, scope.organization_id()).await?;
            Ok(json!({
                "plan": spec.slug,
                "discoveries": { "used": discoveries, "limit": spec.discovery_limit },
                "geo_queries": { "used": geo, "limit": spec.geo_query_limit },
                "monitored_links": { "limit": spec.backlink_limit },
                "projects": { "limit": spec.project_limit },
            }))
        }
        "discover_spots" => {
            let project_id = arg_uuid(&args, "project_id")?;
            let rows = repo::spots::list_spots(pool, scope, project_id).await?;
            Ok(json!(
                rows.into_iter().map(SpotView::from).collect::<Vec<_>>()
            ))
        }
        "explain_spot_score" => {
            let spot_id = arg_uuid(&args, "spot_id")?;
            let s = repo::spots::get(pool, scope, spot_id).await?;
            let signals = SpotSignals {
                keyword_count: s.keyword_count.max(0) as u32,
                domain_rank: s.domain_rank,
                referring_domains: s.referring_domains,
                spam_score: s.spam_score,
                organic_keywords: s.organic_keywords,
                organic_traffic: s.organic_traffic,
            };
            let sc = spot_score(&signals);
            Ok(json!({
                "domain": s.domain,
                "total": sc.total,
                "relevance": sc.relevance,
                "authority": sc.authority,
                "trust": sc.trust,
                "traffic": sc.traffic,
                "breadth": sc.breadth,
            }))
        }
        "geo_citations" => {
            let project_id = arg_uuid(&args, "project_id")?;
            let query_count = repo::geo::count_queries(pool, scope, project_id).await?;
            let rows = repo::geo::domain_citations(pool, scope, project_id).await?;
            let domains = rows
                .into_iter()
                .map(|r| {
                    let signals = GeoSignals {
                        citation_count: r.citation_count.max(0) as u32,
                        query_count: query_count.max(0) as u32,
                        best_rank: r.best_rank.map(|v| v.max(1) as u32),
                        engine_count: r.engine_count.max(0) as u32,
                        placeable: r.placeable,
                        domain: SpotSignals {
                            keyword_count: r.keyword_count.unwrap_or(0).max(0) as u32,
                            domain_rank: r.domain_rank,
                            referring_domains: None,
                            spam_score: r.spam_score,
                            organic_keywords: r.organic_keywords,
                            organic_traffic: r.organic_traffic,
                        },
                    };
                    let gs = geo_score(&signals);
                    json!({
                        "domain": r.cited_domain,
                        "citation_count": r.citation_count,
                        "best_rank": r.best_rank,
                        "engine_count": r.engine_count,
                        "placeable": r.placeable,
                        "score": gs.total,
                    })
                })
                .collect::<Vec<_>>();
            Ok(json!({ "query_count": query_count, "domains": domains }))
        }
        "compare_offers" => {
            let project_id = arg_uuid(&args, "project_id")?;
            let domain = args
                .get("domain")
                .and_then(|v| v.as_str())
                .map(|s| s.trim().to_ascii_lowercase())
                .filter(|s| !s.is_empty())
                .ok_or_else(|| ToolError::BadArgs("missing `domain`".into()))?;
            let offers = repo::spots::offers_for_project(pool, scope, project_id).await?;
            let matched = offers
                .into_iter()
                .filter(|o| o.editor_domain.eq_ignore_ascii_case(&domain))
                .map(OfferView::from)
                .collect::<Vec<_>>();
            Ok(json!(matched))
        }
        _ => Err(ToolError::NotFound),
    }
}
