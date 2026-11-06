# LinkPulz — Jobs & Pipeline Definitions

**Deliverable #3.** Status: draft for alignment, 2026-11-06.
Builds on [`21-ai-link-intelligence-architecture.md`](21-ai-link-intelligence-architecture.md)
and [`22-api-and-mcp-spec.md`](22-api-and-mcp-spec.md). Defines every background job:
trigger, cadence, inputs, **idempotency key**, **cache keys**, provider cost / quota
metric, outputs, and retry. Reuses the durable Postgres queue in `crates/jobs`
(`FOR UPDATE SKIP LOCKED`, backoff, idempotency) — no new queue infra.

Legend: **[reuse]** exists today · **[new]** to add. Enqueue = who schedules it.

---

## 1. Principles

- **Idempotent by construction.** Every job carries an idempotency key; enqueuing an
  equal key within its window is a no-op (`jobs` unique constraint). Keys fold in a
  **date bucket** so the same work re-runs once per day, not per request.
- **Cache before provider.** Any `SerpProvider` / `MetricsProvider` / `GeoProvider`
  call goes through `provider_cache` (doc 21 §6) first. A hit = 0 provider units and is
  **mutualised across tenants** (the margin lever). Writes record `cost_units`.
- **Quota at enqueue, never mid-flight.** Discovery/GEO jobs check the plan meter
  *before* enqueue. Over-quota → `quota_blocked` surfaced as REST `429` — it **never
  deletes data and never pauses monitoring**.
- **Monitoring is sacred.** The monitoring jobs (the retention layer) are never gated
  by acquisition quotas and keep running regardless of plan caps.
- **Scheduler = the cron tick.** `POST /internal/tick` (GitHub Actions, ~10 min) is the
  single scheduler; it enqueues the due/periodic jobs below. No separate scheduler.

## 2. Job catalog

### 2.1 Monitoring (retention layer) — **[reuse]**, unchanged

| Job | Enqueue / cadence | Idempotency key | Provider | Notes |
|---|---|---|---|---|
| `ScheduleDueChecks` | tick | `schedule:{minute_bucket}` | — | enqueues due `CheckSourcePage` |
| `CheckSourcePage` | scheduler/fan-out | `src:{source_url_norm}:{day}` | crawler fetch (own) | one fetch, N backlink evals |
| `CheckTargetUrl` | scheduler | `tgt:{target_url_norm}:{day}` | crawler fetch | — |
| `CheckBacklinkNow` | user ("Check now") | `now:{backlink_id}:{min}` | crawler fetch | jumps queue (prio 100) |
| `CheckWarranties` | tick (daily) | `warranty:{day}` | — | expiry sweep |
| `SendNotification` / `DigestNotifications` | on event / window | `notif:{id}` / `digest:{dest}:{window}` | email/webhook | alert delivery |
| `RecomputeUsage` | tick / post-job | `usage:{org}:{day}` | — | usage counters |
| `RetentionDownsample` | tick (nightly) | `downsample:{day}` | — | prunes `backlink_checks` |
| `ProcessImport` / `ImportRowsChunk` | upload | `import:{upload_id}[:chunk]` | — | CSV ingest |

### 2.2 Acquisition — Spot Finder (Pillars 1 & 3)

| Job | Enqueue / cadence | Idempotency key | Cache key(s) | Quota metric | Status |
|---|---|---|---|---|---|
| `DetectSiteKeywords` | user starts discovery | `kw:{project_id}:{day}` | — (own crawl) | — | **[reuse]** |
| `FindSpots` | after keywords kept | `spots:{project_id}:{kwset_hash}:{locale}:{day}` | `serp:{sha(keyword)}:{locale}` | `discoveries` | **[reuse]** (add cache+quota) |
| `EnrichSpots` | after `FindSpots` | `enrich:{project_id}:{spotset_hash}:{day}` | `metrics:{domain}` | `discoveries` | **[reuse]** (add cache+quota) |
| `RefreshSpotTrends` | tick (daily) + after enrich | `trend:{spot_id}:{day}` | `metrics:{domain}` (trend) | `discoveries` (cheap/cached) | **[new]** → Pillar-3 `decline_flag` |
| `MatchMarketplaces` | after enrich / after catalog refresh | `match:{project_id}:{day}` | — (reads `marketplace_offers`) | — | **[reuse]** (generalise beyond LinkAvista) |

### 2.3 Marketplace catalogs (price comparison) — **[new]**

| Job | Enqueue / cadence | Idempotency key | Notes |
|---|---|---|---|
| `RefreshMarketplaceCatalogs` | tick (daily, per enabled source) | `catalog:{source_id}:{day}` | adapter per source → upsert `marketplace_offers` (normalise site, price, dofollow, content, indexation, retention). Then re-enqueue `MatchMarketplaces` for projects touching changed editor domains. Add-a-source = register in `marketplace_sources` + an adapter impl of `MarketplaceProvider`. |

### 2.4 GEO — AI citations (Pillar 2) — **[new]**

| Job | Enqueue / cadence | Idempotency key | Cache key | Quota metric | Notes |
|---|---|---|---|---|---|
| `CaptureGeo` | user "GEO refresh" + tick (daily/weekly per project) | `geo:{geo_query_id}:{day}` | `ai_overview:{sha(query)}:{locale}` | `geo_queries` | DataForSEO AI Overviews → parse cited sources → resolve domains → upsert `geo_captures` + `geo_citations`; then recompute `geo_spot_links.placeable` by joining `marketplace_offers` + `spots`. |

### 2.5 Digests — weekly opportunities — **[new, extends `DigestNotifications`]**

| Job | Cadence | Idempotency key | Notes |
|---|---|---|---|
| `WeeklyOpportunityDigest` | tick (Mon 08:00 per workspace locale) | `oppdigest:{org}:{isoweek}` | email: new spots + new GEO-cited placeable sources since last week + portfolio changes. One send/week/workspace. |

## 3. Pipeline DAGs

```
DISCOVERY (acquisition, user-triggered)
  DetectSiteKeywords ─▶ FindSpots ─▶ EnrichSpots ─▶ RefreshSpotTrends ─▶ MatchMarketplaces ─▶ shortlist ready
        (own crawl)      (SERP$,cache)  (metrics$,cache)  (trend→decline)   (join offers)
  quota: FindSpots & EnrichSpots decrement `discoveries` at enqueue.

MARKETPLACE (daily)
  RefreshMarketplaceCatalogs[source] ─▶ upsert marketplace_offers ─▶ MatchMarketplaces (affected projects)

GEO (daily/weekly + on demand)        [Pillar 2 — the wedge]
  CaptureGeo[geo_query] ─▶ geo_citations ─▶ recompute geo_spot_links.placeable (join offers+spots)
  quota: CaptureGeo decrements `geo_queries` at enqueue; cache 24–72h TTL.

MONITORING (retention, always on — never quota-gated)
  ScheduleDueChecks ─▶ CheckSourcePage/CheckTargetUrl ─▶ backlink_events ─▶ SendNotification/Digest

ROLL-UP
  RecomputeUsage · RetentionDownsample · WeeklyOpportunityDigest
```

## 4. Cache keys & TTLs (`provider_cache`)

| Kind | Key | TTL | Rationale |
|---|---|---|---|
| SERP organic | `serp:{sha(keyword)}:{locale}` | 24h | daily re-score cadence |
| Domain metrics | `metrics:{domain}` | 7d | authority/traffic move slowly |
| Metrics trend | `metrics_trend:{domain}` | 7d | decline detection |
| AI Overview | `ai_overview:{sha(query)}:{locale}` | 24–72h | GEO answers volatile but costly |

Marketplace offers are **not** in `provider_cache` — they live in `marketplace_offers`,
refreshed by the daily catalog job (full normalised rows, queryable for price compare).

## 5. Idempotency & concurrency

- Keys above fold a **date bucket** (`:{day}` / `:{isoweek}` / `:{minute_bucket}`) so a
  duplicate trigger in-window is a no-op; the next bucket re-runs the work.
- `*_hash` (kwset/spotset) ensures a *changed* input re-runs even within the day.
- Per-domain crawl rate-limiting stays as-is (monitoring). Provider jobs inherit the
  existing backoff/retry curve (`crates/jobs/backoff.rs`); a provider 429/5xx retries
  with backoff and does **not** consume quota on failure.

## 6. Codebase touchpoints

- Extend `crates/jobs/src/kind.rs`: `RefreshSpotTrends`, `RefreshMarketplaceCatalogs`,
  `CaptureGeo`, `WeeklyOpportunityDigest` (+ priorities: user-GEO 15, catalogs/trends/
  digest 10).
- `crates/crawler`: add `geo.rs` (`GeoProvider`, DataForSEO AI-mode) + a thin
  `cache.rs` wrapper used by `serp`/`metrics`/`geo`.
- `crates/db`: repos for `provider_cache`, `geo_*`, `marketplace_sources/offers`,
  quota meter reads.
- `apps/worker`: dispatch arms for the new kinds.
- `apps/api/src/routes/internal_routes.rs`: enqueue the new periodic jobs from the tick.

## 7. Next deliverables

4. Scoring module as isolated unit-tested logic (`crates/core`): `geo_score`, Pillar-3
   `decline_flag`; reuse `spot_score`. — **this is where Phase-level code begins**.
5. Scaffold Phase 0, then Phase 1 (ask before each).
