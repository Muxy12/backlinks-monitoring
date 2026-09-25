//! Backlink graph API (backlink-intelligence §12). Derives a node/edge graph
//! from the project's own backlinks — the monitored domain at the centre (Tier
//! 0), Tier-1 links around it, Tier-2 links reinforcing those. Domain-level by
//! default (scales); url-level on request. No graph tables: it is computed.

use crate::error::ApiError;
use crate::session::current;
use crate::state::AppState;
use axum::Json;
use axum::extract::{Path, State};
use axum_extra::extract::cookie::CookieJar;
use common::Role;
use db::authorize_org;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Serialize)]
pub struct GraphNode {
    pub id: String,
    pub label: String,
    pub domain: String,
    pub tier: i16,
    #[serde(rename = "type")]
    pub node_type: String,
}

#[derive(Serialize)]
pub struct GraphEdge {
    pub source: String,
    pub target: String,
    pub backlink_id: Uuid,
    pub tier: i16,
    pub status: String,
}

#[derive(Serialize)]
pub struct GraphResponse {
    pub view: String,
    pub root: String,
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
    pub truncated: bool,
}

#[derive(Deserialize)]
pub struct GraphQuery {
    /// "domain" (default) or "url".
    pub view: Option<String>,
    pub limit: Option<i64>,
}

pub async fn backlink_graph(
    State(state): State<AppState>,
    Path((org_id, project_id)): Path<(Uuid, Uuid)>,
    jar: CookieJar,
    axum::extract::Query(q): axum::extract::Query<GraphQuery>,
) -> Result<Json<GraphResponse>, ApiError> {
    let (user, _s) = current(&state, &jar).await?;
    let scope = authorize_org(&state.pool, user.id, org_id).await?;
    scope.require(Role::Viewer)?;
    let project = db::repositories::projects::get(&state.pool, &scope, project_id).await?;

    let url_level = q.view.as_deref() == Some("url");
    let view = if url_level { "url" } else { "domain" };
    let limit = q.limit.unwrap_or(2_000).clamp(1, 20_000);
    let rows = db::repositories::graph::edges(&state.pool, &scope, project_id, limit + 1).await?;
    let truncated = rows.len() as i64 > limit;
    let rows: Vec<_> = rows.into_iter().take(limit as usize).collect();

    let root = project.domain.clone();
    let key = |domain: &str, url: &str| -> String {
        if url_level {
            url.to_string()
        } else {
            domain.to_string()
        }
    };

    // Map each edge id -> its source node key, so a Tier-2 edge can point at its
    // parent Tier-1 source node.
    let mut parent_key: HashMap<Uuid, String> = HashMap::new();
    for e in &rows {
        parent_key.insert(e.id, key(&e.source_domain, &e.source_url));
    }

    let mut nodes: HashMap<String, GraphNode> = HashMap::new();
    nodes.insert(
        root.clone(),
        GraphNode {
            id: root.clone(),
            label: root.clone(),
            domain: root.clone(),
            tier: 0,
            node_type: "target".into(),
        },
    );

    let mut edges: Vec<GraphEdge> = Vec::new();
    let mut seen_edges: std::collections::HashSet<(String, String)> =
        std::collections::HashSet::new();

    for e in &rows {
        let node_id = key(&e.source_domain, &e.source_url);
        // Insert / keep the lowest tier for a node.
        nodes
            .entry(node_id.clone())
            .and_modify(|n| {
                if e.tier < n.tier {
                    n.tier = e.tier;
                }
            })
            .or_insert_with(|| GraphNode {
                id: node_id.clone(),
                label: if url_level {
                    e.source_url.clone()
                } else {
                    e.source_domain.clone()
                },
                domain: e.source_domain.clone(),
                tier: e.tier,
                node_type: "backlink".into(),
            });

        // Where this edge points: its parent's source (Tier-2+) or the root.
        let target = match e.parent_backlink_id.and_then(|pid| parent_key.get(&pid)) {
            Some(k) => k.clone(),
            None => root.clone(),
        };
        if seen_edges.insert((node_id.clone(), target.clone())) {
            edges.push(GraphEdge {
                source: node_id,
                target,
                backlink_id: e.id,
                tier: e.tier,
                status: e.status.clone(),
            });
        }
    }

    Ok(Json(GraphResponse {
        view: view.into(),
        root,
        nodes: nodes.into_values().collect(),
        edges,
        truncated,
    }))
}
