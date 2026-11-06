# LinkPulz — Keyed REST API & MCP Server Spec

**Deliverable #2.** Status: draft for alignment, 2026-11-06.
Builds on [`21-ai-link-intelligence-architecture.md`](21-ai-link-intelligence-architecture.md).
Two **new** subscriber-facing surfaces over the existing capabilities: a **keyed REST
API** and an **MCP server** (the AEO/agent play — a launch feature). The existing
cookie-authenticated app API (`/v1/...`) is unchanged; these are additive.

Legend: **[reuse]** wraps an existing internal capability · **[new]** net-new.

---

## 1. Auth & conventions

- **Base URL:** `https://api.linkpulz.com/pub/v1` (keyed) — distinct from the app's
  cookie API. **MCP:** `https://api.linkpulz.com/mcp` (JSON-RPC over streamable HTTP).
- **Auth:** `Authorization: Bearer lp_live_<key>`. Keys live in `api_keys`
  (hashed, `prefix` shown once, `scopes[]`, per-workspace). Revocable. **Provider
  keys (DataForSEO etc.) never leave the server and never appear in any payload.**
- **Scopes:** `read:portfolio`, `read:discovery`, `write:discovery` (trigger jobs),
  `read:geo`, `read:marketplace`, `admin:keys`. MCP tools map to these scopes.
- **Tenancy:** a key is bound to one workspace; every response is workspace-scoped.
  `client_id` / `project_id` filter within it.
- **Quotas:** per-plan (`plan_quotas`). Discovery/GEO endpoints decrement counters and
  return `429 quota_exceeded` when a plan cap is hit — **never deletes data, never
  pauses monitoring**. Headers: `X-LinkPulz-Quota-Remaining`, `X-LinkPulz-Quota-Reset`.
- **Pagination:** cursor (`?cursor=&limit=` ≤100), `{ data, next_cursor }`.
- **Errors:** `{ error: { code, message, detail? } }`, codes stable (`quota_exceeded`,
  `not_found`, `invalid_scope`, `validation_error`).
- **Locale:** project-scoped SERP/GEO locale; override per call with `?locale=en-US`.
- **Idempotency:** `Idempotency-Key` header on POSTs that enqueue jobs.
- **Determinism:** every numeric field traces to a provider call or an explainable
  formula; responses carry `source: "provider" | "computed"` and `as_of` timestamps.

## 2. REST endpoints

### Workspace & context — scope `read:portfolio`
| Method | Path | Notes |
|---|---|---|
| GET | `/clients` | list clients **[reuse]** |
| GET | `/projects?client_id=` | projects (target sites) **[reuse]** |
| GET | `/projects/{id}` | project incl. SERP locale, GSC-connected flag **[reuse]** |
| GET | `/usage` | quota consumption this period **[reuse]** |

### Discovery — thematic + traffic (Pillars 1 & 3)
| Method | Path | Scope | Notes |
|---|---|---|---|
| POST | `/projects/{id}/discoveries` | `write:discovery` | trigger: theme-detect → SERP → score → traffic filter. Returns `discovery_id` (async). **[reuse]** `DetectSiteKeywords`→`FindSpots`→`EnrichSpots` |
| GET | `/discoveries/{discovery_id}` | `read:discovery` | status + progress **[reuse]** |
| GET | `/projects/{id}/spots?sort=score&min_score=&placeable=&max_price=` | `read:discovery` | ranked shortlist: `current_perf`, `potential_perf`, `decline_flag`, authority (secondary), price range **[reuse]** |
| GET | `/spots/{id}` | `read:discovery` | one spot + **explainable score breakdown** (why it ranks) **[reuse+new]** |
| GET | `/spots/{id}/offers` | `read:marketplace` | side-by-side offers: price, dofollow, content_included, indexation_guarantee, retention **[new]** `marketplace_offers` |

### GEO — AI citations (Pillar 2) — scope `read:geo`
| Method | Path | Notes |
|---|---|---|
| GET/PUT | `/projects/{id}/geo-queries` | manage tracked GEO queries **[new]** |
| POST | `/projects/{id}/geo/refresh` | trigger `CaptureGeo` (quota'd) **[new]** |
| GET | `/projects/{id}/geo/citations?query=&engine=` | AI-cited sources (domain, rank, engine, captured_at) + `placeable` flag + matched offer **[new]** |

### Monitoring portfolio (retention layer) — scope `read:portfolio`
| Method | Path | Notes |
|---|---|---|
| GET | `/links?client_id=&project_id=&status=` | monitored links + current status across clients **[reuse]** |
| GET | `/links/{id}` | link detail: status, rel, anchor, source IP, noindex **[reuse]** |
| GET | `/links/{id}/events` | full change history **[reuse]** `backlink_events` |
| POST | `/links` | add a landed link to monitoring (scope `write:discovery`) **[reuse]** |
| GET | `/alerts?since=` | recent change alerts (lost/nofollow/redirect/anchor…) **[reuse]** |

### Webhooks (outbound) — **[reuse]** `alert_destinations`
Existing per-workspace webhook/Slack delivery. Documented event envelope:
`{ event, link_id, project_id, client_id, change, occurred_at }`. Signed with the
key's secret.

## 3. MCP server

- **Transport:** JSON-RPC 2.0 over streamable HTTP at `/mcp` (one endpoint, POST).
  Mirrors the standard MCP `initialize` / `tools/list` / `tools/call` flow.
- **Auth:** same `Bearer lp_live_<key>`; tool availability follows the key's scopes.
- **Design rule:** **read + analysis first**; the only state-changing tools enqueue
  discovery/GEO jobs (quota'd). No destructive tools. Every tool annotates `source`
  and `as_of`; provider keys never surface.
- **Caching:** discovery/GEO tools read `provider_cache` first → cheap, mutualised.

### Tool list (launch set)

| Tool | Scope | Input | Output | Cost |
|---|---|---|---|---|
| `list_clients` | read:portfolio | — | clients[] | free |
| `list_projects` | read:portfolio | `{ client_id? }` | projects[] (target sites, locale) | free |
| `discover_spots` | write:discovery | `{ project_id? \| url? \| theme?, locale?, limit? }` | ranked spots: domain, `current_perf`, `potential_perf`, `decline_flag`, price range, `placeable` | quota (discovery) |
| `explain_spot_score` | read:discovery | `{ spot_id }` | transparent breakdown: theme coverage, position quality, gem signal, authority (secondary), rationale | free (cached) |
| `geo_citations` | read:geo | `{ project_id?, query?, locale?, engine? }` | AI-cited sources (domain, rank, engine), `placeable` flag + matched offer | quota (geo) |
| `compare_offers` | read:marketplace | `{ editor_domain \| spot_id }` | offers side by side: price, dofollow, content, indexation, retention, source | free (cached) |
| `portfolio_health` | read:portfolio | `{ client_id? }` | counts: live / at-risk / lost / nofollow, by client/project | free |
| `list_monitored_links` | read:portfolio | `{ project_id?, client_id?, status? }` | links + current status | free |
| `link_status` | read:portfolio | `{ link_id }` | status + recent event history | free |
| `recent_alerts` | read:portfolio | `{ since?, project_id? }` | change alerts | free |
| `usage` | read:portfolio | — | quota consumption + plan caps | free |

**Deferred (Phase 3, gated write scopes):** `add_monitored_link`, `create_campaign`,
`export_report`.

### Example `tools/call`
```json
{ "method": "tools/call",
  "params": { "name": "discover_spots",
    "arguments": { "project_id": "prj_…", "locale": "en-US", "limit": 25 } } }
```
→ returns ranked spots, each with `source` and `as_of`; a hit reuses `provider_cache`
so no provider units are spent.

## 4. Mapping to the codebase

- **Keyed REST router:** new `apps/api/src/routes/pub_routes.rs` (+ `api_keys` auth
  middleware) composing existing repo calls; GEO/offers endpoints are net-new.
- **MCP router:** new `apps/api/src/routes/mcp_routes.rs` implementing JSON-RPC +
  `tools/*`; each tool delegates to the same repo/service functions as REST (one
  service layer, two transports). Original implementation — not derived from any
  third-party MCP code.
- **Auth:** `api_keys` table (see doc 21 §4.2); hash with the existing Argon2/again
  util; scope check in middleware.
- **Quotas:** enforced in the shared service layer so REST and MCP share one meter.

## 5. Next deliverables

3. Job/pipeline definitions (triggers, idempotency, cache keys) — expands doc 21 §5.
4. Scoring module as isolated unit-tested logic (`crates/core`: `geo_score`, Pillar-3
   `decline_flag`; reuse `spot_score`).
5. Scaffold Phase 0, then Phase 1 (ask before each).
