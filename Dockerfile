# LinkPulz API + worker container (ADR-0002: long-running Rust services, NOT on
# Vercel). One image holds all three binaries — `server` (the HTTP API),
# `worker` (the monitoring loop) and `scheduler` (a one-shot cron tick) — and the
# process is selected at run time (see fly.toml `[processes]`).
#
# TLS is rustls + webpki-roots everywhere (sqlx and reqwest), so the runtime
# needs no OpenSSL; ca-certificates is added only for good measure.

# syntax=docker/dockerfile:1

# ─── Builder ──────────────────────────────────────────────────────────────────
# Pinned to the repo's toolchain (rust-toolchain.toml → 1.98.1) so a build here
# matches CI exactly. buildpack-deps base provides the C toolchain `ring` needs.
FROM rust:1.98.1-bookworm AS builder
WORKDIR /build

# Copy the workspace. Only the cargo members + the embedded migrations are
# needed (the Next.js app in apps/web is not a cargo member).
COPY Cargo.toml Cargo.lock rust-toolchain.toml ./
COPY crates ./crates
COPY apps/api ./apps/api
COPY apps/worker ./apps/worker
COPY migrations ./migrations

# Build the three release binaries. Cache the cargo registry and target dir
# across builds; copy the finished binaries out of the (cached) target so the
# next stage can take them.
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/build/target \
    cargo build --release --locked \
        --bin server --bin worker --bin scheduler \
 && mkdir -p /out \
 && cp target/release/server target/release/worker target/release/scheduler /out/

# ─── Runtime ──────────────────────────────────────────────────────────────────
FROM debian:bookworm-slim AS runtime
RUN apt-get update \
 && apt-get install -y --no-install-recommends ca-certificates \
 && rm -rf /var/lib/apt/lists/*

# Run as a non-root user.
RUN useradd --system --uid 10001 --home /app backlio
WORKDIR /app
COPY --from=builder /out/server /app/server
COPY --from=builder /out/worker /app/worker
COPY --from=builder /out/scheduler /app/scheduler
USER backlio

ENV PORT=8080
EXPOSE 8080

# Default to the API; fly.toml's `[processes]` overrides this for the worker.
CMD ["/app/server"]
