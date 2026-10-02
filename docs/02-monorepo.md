# 02 — Monorepo layout

**Spec refs:** §16, §17. **Decision:** [ADR-0001](adr/0001-modular-monolith-cargo-workspace.md).

```
LinkPulz/
├── Cargo.toml                  # Rust workspace root; [workspace.dependencies] pins every version
├── pnpm-workspace.yaml
├── package.json
├── rust-toolchain.toml         # pinned toolchain — CI and local must agree
├── docker-compose.yml          # local Postgres (§15 option A)
├── .env.example                # §43. Never a real secret.
├── README.md
│
├── apps/
│   ├── web/                    # Next.js 15 App Router + TS + Tailwind + shadcn/ui → Vercel
│   ├── api/                    # Axum HTTP layer
│   │   ├── src/
│   │   │   ├── lib.rs          # `pub fn app(state: AppState) -> Router` — the whole API
│   │   │   ├── routes/         # thin handlers: validate → call crate → map to HTTP
│   │   │   ├── middleware/     # auth, csrf, rate limit, request id, body limit
│   │   │   ├── extract/        # OrgScope extractor, validated JSON
│   │   │   ├── error.rs        # ApiError → HTTP status + RFC 7807 body
│   │   │   └── bin/
│   │   │       ├── server.rs   # binds 0.0.0.0:$PORT  (container runtime, any host)
│   │   │       └── vercel.rs   # vercel_axum adapter  (feature = "vercel-runtime")
│   │   ├── tests/              # tenant_isolation.rs, auth.rs, backlinks.rs …
│   │   ├── Dockerfile.vercel
│   │   └── vercel.json
│   └── worker/
│       ├── src/
│       │   ├── lib.rs
│       │   └── bin/
│       │       ├── worker.rs       # job loop + reaper + internal scheduler timer
│       │       └── scheduler.rs    # one-shot tick, for cron-style invocation
│       └── Dockerfile
│
├── crates/
│   ├── core/           # domain types + backlink state machine. NO sqlx, reqwest, axum.
│   ├── db/             # pool construction, repositories (tenant-scoped), migration runner
│   ├── crawler/        # SSRF guard, fetch, parse, normalize, CrawlResult
│   ├── jobs/           # queue, claim/complete/retry, handlers, scheduler
│   ├── auth/           # Argon2id, sessions, CSRF, OrgScope construction
│   ├── billing/        # Stripe plans, entitlements, webhooks
│   ├── notifications/  # channel trait + in-app/email impls
│   └── common/         # config, errors, tracing, ids, clock
│
├── migrations/         # SQLx, forward-only, numbered
├── docs/               # this directory
└── .github/workflows/  # ci.yml, migrate-production.yml
```

## Dependency direction

```
apps/api ─┐
          ├─► jobs ─► crawler ─┐
apps/worker┘        │           ├─► core ─► common
                    └─► db ─────┘
          ├─► auth ─► db
          ├─► billing ─► db
          └─► notifications ─► db
```

Downward only, no cycles. `common` is the leaf. `core` sits just above it and knows nothing about
I/O — which is why the state machine ([08](08-backlink-state-machine.md)) can be exhaustively
property-tested with no database and no network.

## Why two package managers in one tree

`Cargo.toml` (Rust workspace) and `pnpm-workspace.yaml` (JS workspace) coexist at the root and do
not know about each other. That is fine and deliberate — `apps/web` talks to the Rust API over
HTTP, not over a build dependency. The only shared artifact is the **OpenAPI document** the API
emits, from which the web app generates its TypeScript client. That generation is a CI step, so a
backend contract change that breaks the frontend fails the build rather than production.

## Toolchain pinning

`rust-toolchain.toml` pins the exact Rust version so `cargo clippy` gives identical results locally,
in CI, and in the Docker build. A clippy lint that appears only in CI is a tax on every PR.

## Commands

```bash
# Rust
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo run -p api --bin server
cargo run -p worker --bin worker

# Database
docker compose up -d postgres
sqlx migrate run
sqlx migrate info

# Frontend
pnpm install
pnpm --filter web dev
pnpm --filter web lint
pnpm --filter web typecheck
pnpm --filter web test
```
