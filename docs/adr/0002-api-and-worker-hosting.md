# ADR-0002 — Where the API and the worker actually run

- **Status:** Accepted
- **Date:** 2026-09-24
- **Spec refs:** §7, §8, §9, §54
- **Research:** [01 §1](../01-research-findings.md#1-vercel-support-for-rust--axum)

## Context

The spec requires Vercel for "frontend and web-facing application deployment" and asks us to
evaluate current Vercel options for Rust/Axum in this preference order:

1. Official Vercel Rust runtime where appropriate
2. Vercel-supported Rust/Axum deployment
3. Vercel container deployment if needed

Verified on 2026-09-24: both the native Rust runtime (public **beta**, permissions-gated,
`[[bin]]`-per-handler via `vercel_runtime` 2.4.1) and the container runtime
(`Dockerfile.vercel` + `runtime: "container"`) exist and are real.

## Decision

### The API: Vercel **container runtime**, with the native runtime kept reachable

`apps/api` is a library crate (`api::app() -> axum::Router`) plus two thin binaries:

| Binary | Target | Entry |
|---|---|---|
| `api-server` | Vercel container runtime, and any container host | binds `0.0.0.0:$PORT`, serves `app()` |
| `api-vercel` | Vercel native Rust runtime (beta) | `vercel_axum` adapter wrapping `app()` |

`api-vercel` is behind a `vercel-runtime` cargo feature so its dependencies do not enter the default
build. Both binaries are ~30 lines. Neither contains a business rule.

Why the container is the default rather than option 1 in the spec's order:

- Our API is **one** Axum `Router` with shared middleware and a shared `PgPool`. The native
  runtime's `[[bin]]`-per-handler model fights that shape; collapsing to a single catch-all binary
  to avoid the fight means we get the container's programming model without the container's
  connection-lifetime benefits.
- The native runtime is **beta and permissions-gated** (open item R2). A production MVP should not
  have its only deployment path behind a beta gate.
- The same `Dockerfile.vercel` runs on Fly.io, Railway, Render or a VPS unchanged. That is the
  "keep crawler and domain infrastructure portable" requirement (§57.14) applied to the API too.

This is spec option 3, chosen over option 1 deliberately and reversibly: flipping to the native
runtime is a build-target change, not a rewrite. If R2 comes back "beta access granted and we want
it," we ship `api-vercel` and delete nothing.

### The worker: **not on Vercel**

The crawler worker holds a global concurrency semaphore, per-domain rate limiters with backoff and
jitter, and an in-flight crawl budget, for minutes at a time. Vercel Functions — Fluid or container
— are request-scoped autoscaling compute. That is a mismatch, not a limitation to work around.

The worker runs as a container on a long-running host. **Recommended: Fly.io** (cheap, global
regions close to crawl targets, simple `fly.toml` scaling). Railway, Render or a VPS are equivalent;
the decision is deliberately low-stakes because the worker is a plain container.

This does not contradict the spec. §4 scopes Vercel to *"frontend and web-facing application
deployment"* — the worker is neither.

### The scheduler: Vercel Cron → API → enqueue

The scheduler is short, cheap and idempotent: find due backlinks, group them, enqueue jobs. That
fits a function. A Vercel Cron hits `POST /internal/scheduler/tick` (shared-secret authenticated)
on a schedule; the handler takes `pg_advisory_xact_lock`, enqueues, and returns.

The worker **also** runs the same scheduler tick on an internal timer, so the system keeps
monitoring if Vercel Cron is unavailable. Both paths take the same advisory lock, so a double fire
is a no-op rather than a double enqueue.

## Resulting topology

```
                          USERS
                            │
                            ▼
                      VERCEL CDN
                            │
              ┌─────────────┴─────────────┐
              ▼                           ▼
       Next.js (apps/web)          Rust API (apps/api)
       Vercel, App Router          Vercel container runtime
              │                           │
              └─────────────┬─────────────┘
                            ▼
                  Neon PostgreSQL (pooled)
                            ▲
        ┌───────────────────┼───────────────────┐
        │                   │ (direct)          │
 Vercel Cron ──► /internal/scheduler/tick       │
                                                │
                                      Rust worker (apps/worker)
                                      Fly.io container, long-running
                                                │
                                                ▼
                                          Rust crawler
                                                │
                                                ▼
                                         External sites
```

## Consequences

- Two hosting providers instead of one. Accepted: the alternative is a crawler that cannot hold
  rate-limit state, which would mean hammering publisher sites (§28).
- `Dockerfile.vercel` and the worker `Dockerfile` share a `cargo-chef` base stage, so CI builds the
  dependency layer once.
- Deployment-specific code is confined to `apps/api/src/bin/` and `apps/worker/src/bin/`.
- Open item R2 (native runtime beta access) does not block anything.
