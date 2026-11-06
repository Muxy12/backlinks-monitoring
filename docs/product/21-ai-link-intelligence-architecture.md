# LinkPulz — AI-era Link Intelligence: Architecture & Data Model

**Refactor baseline (Deliverable #1).** Status: draft for alignment, 2026-11-06.
This evolves the existing monitoring MVP toward the new positioning **incrementally** —
the monitoring engine is kept as the retention layer; only what blocks the new
positioning is rewritten. It is not a green-field rebuild.

Companion docs (authoritative for what already ships): [`01-prd.md`](01-prd.md),
[`02-trd.md`](02-trd.md), [`05-backend-schema.md`](05-backend-schema.md),
[`06-implementation-plan.md`](06-implementation-plan.md).

---

## 0. Decisions locked

1. **Stack: keep Rust + Postgres queue, adapt in place.** The §6 Node/Python +
   Redis/BullMQ suggestion is **rejected**: the reliability asset is
   `crates/core` — a pure, I/O-free, exhaustively unit-tested domain core
   (state machine, scoring, monitoring). Rewriting it would reintroduce the exact
   "missed lost-link = churn" risk the spec forbids. Redis is not needed at this
   scale; the Postgres `FOR UPDATE SKIP LOCKED` queue stays.
2. **GEO pillar first engine: DataForSEO AI Overviews / AI-mode.** Reuse the
   provider we already pay for (same `DATAFORSEO_*` creds as SERP + metrics).
   Perplexity/others slot in later behind a `GeoProvider` interface.
3. **Doc-first:** this document is the baseline before any Phase-0/1 code.

## 1. Positioning delta

- **Before:** "monitor your backlinks" (retention tool).
- **After:** "find link spots that rank in Google **and** get cited by AI, only on
  sites with real living traffic — then monitor what you land across every client."
- The monitor becomes the **retention layer under an acquisition-intelligence
  product**, not the headline. Public copy: "link opportunity discovery / relevant
  placements", never "buy backlinks" (protects our own SEO/AEO).

## 2. The three pillars → current coverage

| Pillar | State | Where it lives today |
|---|---|---|
| **1. Thematic** (theme detection → SERP → relevance, `current`/`potential` + gems) | ✅ Built | jobs `DetectSiteKeywords`→`FindSpots`; `crates/crawler/{sitescan,serp}.rs` (**DataForSEO SERP integrated**, env-gated); scoring in `crates/core/{spots,spot_score}.rs` (pure, tested) |
| **2. Cited by AI / GEO** (the wedge) | ❌ **Net-new** | — |
| **3. Real traffic / health** | 🟡 Partial | `EnrichSpots` + `crates/crawler/metrics.rs` (`MetricsProvider`); **decline/fake-traffic filter to add** |
| **Price comparison** (same editor across marketplaces) | 🟡 Partial | `MatchMarketplaces` + `MarketplaceProvider` (`providers_from_env`); **only LinkAvista (FR) wired** |
| **Monitoring + alerts + digest** (retention) | ✅ Built & live | state machine, `backlink_events`, `alert_destinations`, weekly digest |

## 3. Target architecture (keep Rust; add 4 things)

```
apps/web (Next.js)                      ── UI: acquisition shortlist, GEO, portfolio
apps/api (axum)                         ── REST (cookie) + NEW keyed REST + NEW MCP router
apps/worker                             ── job runners
crates/core        (pure domain)        ── + geo scoring (NEW), existing spot/monitor logic
crates/crawler     (I/O, providers)     ── SerpProvider, MetricsProvider, MarketplaceProvider
                                           + GeoProvider (NEW), + provider cache wrapper (NEW)
crates/jobs        (queue model)        ── + CaptureGeo / RefreshCatalogs / RefreshTrends (NEW)
crates/db          (sqlx repos)         ── + provider_cache, geo_*, api_keys, marketplace_* (NEW)
crates/billing     (Stripe)             ── + hard per-plan quotas (NEW enforcement)
Postgres (Neon)                         ── durable queue + data + mutualised cache
```

**Provider interfaces (swappable, env-gated — pattern already in `crates/crawler`):**
- `SerpProvider` — DataForSEO Google organic *(exists)*
- `MetricsProvider` — authority/traffic *(exists)*
- `MarketplaceProvider` — catalog adapters, `providers_from_env` *(exists; add sources)*
- **`GeoProvider` (NEW)** — `ai_overview(query, locale) -> Vec<Citation>` via DataForSEO AI-mode.

**The 4 net-new capabilities:** (1) GEO pillar, (2) MCP server, (3) mutualised
cross-tenant provider cache, (4) hard per-plan discovery/GEO quotas.

## 4. Data model

### 4.1 Existing (grouped — see `05-backend-schema.md` for columns)

- **Tenancy:** `organizations → clients → projects → backlinks`; members/roles/sessions.
- **Monitoring:** `backlinks, backlink_checks, backlink_events, link_occurrences`,
  classification, `monitoring_level`, `daily_backlink_snapshot`.
- **Acquisition:** `spot_keywords, spots, spot_metrics, spot_marketplace, spot_forums`;
  `competitors, discovery_seeds`; campaigns/outreach/reclamation.
- **Jobs/alerts:** `jobs`, `notifications`, `alert_destinations`. **GSC/Impact:** `gsc_*`,
  `backlink_impact`. **Billing:** plans/subscriptions/usage.

### 4.2 Net-new tables (proposed)

```
-- (1) Mutualised, cross-tenant provider cache — §7 margin lever.
provider_cache(
  id, provider text,            -- 'dfs_serp' | 'dfs_metrics' | 'dfs_ai_overview' | ...
  cache_key text,               -- canonical hash of {kind, query|domain, locale}
  locale text, payload jsonb,
  cost_units int, fetched_at timestamptz, expires_at timestamptz, hits int,
  UNIQUE(provider, cache_key))  -- reads are tenant-agnostic; never store PII here

-- (2) GEO pillar.
geo_queries(id, organization_id, project_id, query text, locale text,
            engine text, enabled bool, created_at)            -- the GEO queries we track
geo_captures(id, organization_id, project_id, geo_query_id,
             engine text, captured_at, cache_ref)             -- one engine answer snapshot
geo_citations(id, organization_id, project_id, geo_query_id, capture_id,
              cited_url text, cited_domain text, rank int, captured_at)
-- join GEO-cited domains to buyable availability + "placeable" flag:
geo_spot_links(id, organization_id, project_id, cited_domain text,
               spot_id uuid NULL, marketplace_offer_id uuid NULL,
               placeable bool, reason text, updated_at)

-- (3) Normalised multi-source marketplace offers (price comparison).
marketplace_sources(id, slug, display_name, locale, enabled, last_import_at)
marketplace_offers(id, source_id, editor_domain text, price_cents int, currency,
                   dofollow bool, content_included bool, indexation_guarantee bool,
                   retention_months int, offer_url text, imported_at)
                   -- spots join offers by editor_domain → side-by-side compare

-- (4) Subscriber keys (REST + MCP auth), plan-scoped.
api_keys(id, organization_id, name, key_hash, prefix, scopes text[],
         last_used_at, created_at, revoked_at)
```

Relationships: `geo_queries` 1─N `geo_captures` 1─N `geo_citations`;
`geo_citations.cited_domain` → `geo_spot_links` → (`spots` | `marketplace_offers`);
`marketplace_sources` 1─N `marketplace_offers`; `spots.editor_domain` ↔ `marketplace_offers`.
*(The existing `spot_marketplace` table is the LinkAvista-only precursor to
`marketplace_offers`; migrate it under the normalised model.)*

## 5. Jobs / pipeline delta (full defs = Deliverable #3)

New `JobKind`s (extend `crates/jobs/src/kind.rs`):
- `RefreshMarketplaceCatalogs` — daily per source → `marketplace_offers` (idempotent upsert).
- `CaptureGeo` — per project's enabled `geo_queries` → `GeoProvider` (cache-wrapped) →
  `geo_captures` + `geo_citations` → recompute `geo_spot_links.placeable`.
- `RefreshSpotTrends` — traffic/keyword trend → Pillar-3 `decline_flag`.
- Existing `FindSpots`/`EnrichSpots` gain a **cache read/write** around every provider
  call, and a **quota check at enqueue** (block, don't delete).

Cache keys (canonical, locale-scoped): `serp:{sha(keyword)}:{locale}`,
`metrics:{domain}`, `ai_overview:{sha(query)}:{locale}`. TTLs per kind.

## 6. Margin: mutualised cache + quotas (§7, non-negotiable)

- Every `SerpProvider`/`MetricsProvider`/`GeoProvider` call goes through
  `provider_cache` first; a hit costs 0 provider units and is reused across tenants.
- `plan_quotas(plan) = { discoveries, geo_queries, monitored_links, projects, users }`.
  Enforced **at job enqueue**: over-quota → `quota_blocked` (never deletes data, never
  pauses monitoring). Ensures `provider_cost(plan) < price(plan)`.

## 7. Adapted phase sequencing (reuse-first; ask before each)

- **Phase 0 (small):** `provider_cache` + quota enforcement + project-level SERP locale
  + confirm Pillar-3 decline filter.
- **Phase 1:** re-skin existing Spot Finder as *thematic + traffic + price-compare*
  acquisition; `marketplace_sources/offers` + ≥1 international adapter; shortlist →
  existing monitoring.
- **Phase 2 (wedge):** GEO pillar (DataForSEO AI Overviews) + **MCP server**.
- **Phase 3:** more GEO engines, Chrome extension, white-label reporting, keyed REST API.

## 8. Next deliverables (in order)

2. REST API spec (keyed) + **MCP tool list**.
3. Job/pipeline definitions (triggers, idempotency, cache keys) — expands §5.
4. Scoring module as isolated, unit-tested logic — extend `crates/core` (`geo_score`,
   Pillar-3 `decline_flag`), reuse `spot_score`.
5. Scaffold Phase 0, then Phase 1.

## 9. Guardrails

- Référencime is inspiration only — never reproduce its code/assets/data/copy.
- Never fabricate SERP positions, traffic, GEO citations, or scores: every number
  traces to a provider call or an explainable formula.
- Provider keys stay server-side; never exposed to the client or the MCP payload.
- Public copy: "link opportunity discovery / relevant placements", not "buy backlinks".
