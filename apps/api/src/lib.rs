//! Backlio HTTP API — Axum router and middleware (docs/09-api.md).
//!
//! `app()` builds the whole router so both binaries (container `server` and the
//! Vercel adapter `api-vercel`) share one definition (ADR-0002). Skeleton only;
//! routes land in Phase 1 (docs/13-roadmap.md).
