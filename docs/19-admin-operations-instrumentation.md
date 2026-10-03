# 19 — Admin operations: telemetry status & instrumentation spec

What the founder-ops backoffice can answer **today**, what it **can't**, and the
exact backend work to close each gap. Principle: the console never fabricates a
metric — where telemetry is missing it says so and points here.

This maps the 16 "every-morning" operator questions to their real data source.
Verified against the schema (`migrations/`) and code (`crates/`, `apps/`) — see
file/table references per item.

**Legend**
- ✅ **Data ready** — the data already exists in Postgres; only a **read endpoint** is missing (a quick win, no new instrumentation).
- 🔧 **Needs instrumentation** — the data is **not captured** yet (dead column, unwritten table, un-integrated provider).
- 🟢 **Live now** — already shown in the admin overview.

## The 16 questions

| # | Question | Status | Source / what's needed |
|---|----------|--------|------------------------|
| 8 | New users? | 🟢 Live | Derived from `users.created_at` (admin overview: "+N new · 7d") |
| 9 | Users activating? | 🟢 Live | Workspaces with ≥1 backlink (overview: "Activated %") |
| 10 | Who is paying? | ✅ Data ready | `subscriptions.status IN (active,trialing,past_due)`; exclude admin-comped `stripe_customer_id='admin:*'` |
| 11 | Who canceled? | ✅ list / 🔧 timing | `status='canceled'` is queryable; **when** needs `canceled_at` (column exists, webhook never writes it) |
| 1 | Is LinkPulz working? | ✅ Data ready | Surface `/internal/tick` health (last-tick time, processed/failed) |
| 2 | Checks running? | ✅ Data ready | `backlink_checks.checked_at` window — count today |
| 3 | Queue healthy? | ✅ Data ready | `jobs` table: pending/processing/dead + `jobs_stuck_idx`/`jobs_dead_idx` |
| 4 | Checks failing unusually? | ✅ Data ready | `backlink_checks.error_kind` / `found=false` over time |
| 5 | DataForSEO working? | 🔧 Needs instrumentation | **Not integrated** — no provider, no request/error logging |
| 6 | External providers healthy? | 🔧 Needs instrumentation | Only Resend + Stripe outbound; no provider-health store |
| 7 | Emails delivered? | ✅ sends / 🔧 delivery | `alert_deliveries` logs send ok/err; **delivered/bounced** needs a Resend webhook |
| 12 | Reaching plan limits? | ✅ Data ready | Limits in `crates/billing/src/plans.rs` (`backlink_limit`…) vs live counts |
| 13 | High-usage customers? | 🔧 Needs instrumentation | `usage_records` table exists but is **never written** |
| 14 | Provider/API usage? | 🔧 Needs instrumentation | Same — no metering written; no provider cost |
| 15 | Support problems? | 🟢 partial | Search → account snapshot (overview). Deeper (failed jobs per workspace) = endpoints in #2/#3 |
| 16 | Important events since last visit? | 🟢 partial / 🔧 | Audit log = **admin actions only**. Signups/cancels/failure-spikes feed = new event aggregation |

## ✅ Quick wins — data exists, add a read endpoint (no new instrumentation)

These are cross-tenant admin reads over data already persisted. Each is one query
+ one `/v1/admin/...` handler (gate with the existing `require_admin`).

1. **Queue health** — `GET /v1/admin/ops/queue` → `{pending, processing, dead, stuck, oldest_pending_age}`.
   Source: `jobs` (`migrations/0008_jobs.sql`); `jobs_stuck_idx` (processing with old `locked_at`), `jobs_dead_idx` (`status='dead'`). A `jobs::count_pending` helper exists (`repositories.rs:~1270`, test-only) — add dead/stuck/oldest.
2. **Monitoring health** — `GET /v1/admin/ops/monitoring?window=24h` → `{checks, failed, timeouts, avg_duration_ms, lost_detected}`.
   Source: `backlink_checks` (`migrations/0006_history.sql`): `found`, `error_kind`, `duration_ms`, `checked_at`. Caveat: no worker attribution (no worker column on checks — only `jobs.locked_by`); "checks per worker" needs instrumentation. Retention design = 90 days (the `RetentionDownsample` job has no handler yet, so rows currently accumulate).
3. **Revenue** — `GET /v1/admin/billing` → `{mrr, paying, trialing, past_due, canceled, by_plan}`.
   Source: `subscriptions` (`migrations/0010_billing.sql`) joined to the price catalog `crates/billing/src/plans.rs` (`price_cents`: starter $19 / pro $49 / agency $99 / agency_plus $199). **MRR must exclude admin-comped rows** (`set_plan` writes `status='active'`, `stripe_customer_id='admin:{org}'`).
4. **Plan-limit usage** — `GET /v1/admin/usage/limits` → per-workspace `{backlinks, backlink_limit, pct}`.
   Source: live `count(*) FROM backlinks` per org (admin list already does this) vs `plans.rs` `backlink_limit`.
5. **Email (alerts)** — `GET /v1/admin/ops/email?window=24h` → `{sent, failed}` from `alert_deliveries` (`migrations/0024`, `kind='email'` via `alert_destinations`, `migrations/0025`).
6. **Scheduler/system** — `GET /v1/admin/ops/system` → last-tick time + the `/internal/tick` return counts (`{scheduled_jobs, processed, warranty_alerts, delivered, partial}`), persisted per tick.

## 🔧 Needs instrumentation — data not captured yet

1. **Churn / renewal / trial timing.** `subscriptions` has `current_period_end`, `canceled_at`, `trial_end`, `cancel_at_period_end`, but `apply_event` (`apps/api/src/routes/billing_routes.rs:~181`) and the repo writers (`upsert_subscription` / `update_subscription_by_stripe_id`, `repositories.rs:~2666/~2696`) only write `plan`, `status`, Stripe ids. **Fix:** populate these four columns from the Stripe `customer.subscription.*` events. Unblocks "who canceled *when*", renewal dates, trials ending.
2. **Email delivery / bounce / complaint.** `resend::send_email` (`crates/notifications/src/lib.rs`) is fire-and-forget and discards the Resend message id; no inbound webhook. **Fix:** store the Resend id, consume Resend's `email.delivered`/`email.bounced`/`email.complained` webhook into a log. Also: transactional auth emails (`auth_routes.rs:~52`) are unlogged, and `notifications.emailed_at` (`migrations/0011`) is never written.
   *Doc drift to fix:* migration 0025/docs say `RESEND_API_KEY`/`RESEND_FROM`; code reads `EMAIL_PROVIDER_API_KEY`/`EMAIL_FROM`.
3. **Per-workspace usage metering & API/provider cost.** `usage_records` (`migrations/0010`: `backlinks_monitored`, `checks_performed`, `http_requests`, `bytes_downloaded`) is **never written** — the `RecomputeUsage` job kind (`crates/jobs/src/kind.rs`) has **no worker handler** (8 of 11 kinds don't). **Fix:** implement `RecomputeUsage` and enqueue it from the tick. External **provider cost** (#5/#13/#14) is moot until a paid data provider (e.g. DataForSEO) is integrated — none is today.

## Notes / skeptical flags (so the console never overclaims)
- Schema-present but **code-dead** (do not surface as working): `subscriptions` period/cancel/trial columns, `usage_records`, `notifications.emailed_at`, `backlink_checks.raw`/`redirect_chain`.
- 8 of 11 job kinds have no handler (`CheckTargetUrl`, `ProcessImport`, `ImportRowsChunk`, `SendNotification`, `DigestNotifications`, `RecomputeUsage`, `RetentionDownsample`).
- Derived overview metrics (new-signups, activation, plan mix) are computed client-side from the loaded lists; at scale, move them to aggregate endpoints (the overview shows a coverage note when a list is incomplete).
- Security: every new admin endpoint must go through the existing `require_admin` guard (`admin_routes.rs`); see [05 §2](05-tenant-isolation.md).
