# LinkPulz

**Backlink monitoring and link-building operations for SEO agencies, link builders and consultants.**

Track every backlink you build. Monitor links, costs, campaigns and warranties in one place. Get
notified when a backlink disappears, changes or breaks.

This is not an Ahrefs/Semrush competitor and does not try to index the web. It monitors the
backlinks you already know about, and tells you which ones need attention today.

---

## Stack

| Layer | Technology |
|---|---|
| Frontend | Next.js (App Router), TypeScript, Tailwind, shadcn/ui → **Vercel** |
| API | **Rust** + Axum → Vercel container runtime |
| Worker / crawler | **Rust** + Tokio → long-running container (Fly.io) |
| Database | **Neon** PostgreSQL via SQLx |
| Queue | PostgreSQL (`FOR UPDATE SKIP LOCKED`) — no Redis |
| Billing | Stripe |

Core business logic — monitoring, crawling, state transitions, authorization — is in Rust.

## Status

**Phase 0 (architecture & design) complete. Phase 1 (foundation) not yet started.**

No application code has been written yet, so no build or test claims are made. See
[docs/13-roadmap.md](docs/13-roadmap.md).

## Documentation

Start at **[docs/00-index.md](docs/00-index.md)**.

The design is grounded in verified facts rather than assumptions — see
[docs/01-research-findings.md](docs/01-research-findings.md), which records what was checked, when,
against which source, and which questions can only be settled by a test we have not run yet.

## Architecture

```
                          USERS
                            │
                            ▼
                      VERCEL CDN
                            │
              ┌─────────────┴─────────────┐
              ▼                           ▼
       Next.js (apps/web)          Rust API (apps/api)
              │                           │
              └─────────────┬─────────────┘
                            ▼
                  Neon PostgreSQL (pooled)
                            ▲
        ┌───────────────────┼──────────────────┐
        │ (direct)          │                  │
 Vercel Cron ──► /internal/scheduler/tick      │
                                     Rust worker (apps/worker)
                                               │
                                               ▼
                                         Rust crawler
                                               │
                                               ▼
                                        External sites
```

Why the worker is not on Vercel: [ADR-0002](docs/adr/0002-api-and-worker-hosting.md).

## Local development

Requires: Rust (see `rust-toolchain.toml`), Node 22+, pnpm, Docker.

```bash
cp .env.example .env
docker compose up -d postgres
cargo sqlx migrate run
cargo run -p api --bin server       # :8080
cargo run -p worker --bin worker    # separate terminal
pnpm --filter web dev               # :3000
```

No production credentials are needed locally.

## Checks

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
pnpm --filter web lint && pnpm --filter web typecheck && pnpm --filter web test
```

## Repository layout

```
apps/web      Next.js frontend
apps/api      Axum HTTP layer (thin — wiring, not rules)
apps/worker   job loop, scheduler, crawler host
crates/       core · db · crawler · jobs · auth · billing · notifications · common
migrations/   SQLx migrations
docs/         architecture documentation
```

`crates/core` holds the backlink state machine and has no I/O dependencies, so it is fully testable
without a database or network. See [docs/02-monorepo.md](docs/02-monorepo.md).
