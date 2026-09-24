# 11 — Security risk register

**Spec refs:** §26, §45, §50, §56.14

Ordered by severity. Each risk names its control **and the test that proves the control works** —
a control with no test is an intention.

---

## S1 — SSRF via user-supplied URLs · **Critical**

The defining risk of this product. Users supply arbitrary URLs; we fetch them from infrastructure
that sits inside a network with a cloud metadata endpoint, and the public checker (§50) exposes that
capability to anonymous callers.

**Attack shapes:** `http://169.254.169.254/latest/meta-data/iam/security-credentials/` ·
`http://localhost:8080/internal/…` · `http://[::ffff:127.0.0.1]/` · a public DNS name resolving to
`10.0.0.5` · a 200-OK page that redirects hop 3 to a private address · DNS rebinding between the
check and the connect.

**Controls** ([06 §2](06-crawler.md#2-ssrf-protection)): scheme allowlist · reject credentials in
URL · port allowlist · explicit DNS resolution with **every** returned address validated · v4-mapped
and 6to4 unwrapped and re-checked · **connect by pinned validated IP** · `redirect::Policy::none()`
with our own hop loop re-running the full guard · hostname denylist · response size cap · one shared
guard for the authenticated crawler and the public tool.

**Proof:** `crates/crawler/tests/ssrf.rs`, listed in [06 §7](06-crawler.md#7-test-fixtures).
This suite is a merge blocker.

**Residual:** a genuinely public IP that fronts something sensitive is not distinguishable from any
other public IP. Accepted; mitigated at the network layer by running the worker with **egress-only**
network policy and no IAM role worth stealing.

---

## S2 — Cross-tenant data access (IDOR) · **Critical**

Every route takes an org id. One handler that forgets to verify membership exposes another agency's
client list, costs and publisher relationships — commercially sensitive data.

**Controls:** four layers in [05](05-tenant-isolation.md) — unconstructable `OrgScope`,
repository-injected `WHERE`, composite FKs in the schema, table-driven route tests. 404 not 403.

**Proof:** `apps/api/tests/tenant_isolation.rs`, driven from the route table so that a new route
without a test **fails the suite**.

---

## S3 — The public checker as free crawler / DoS amplifier · **High**

§50 warns about exactly this: an unauthenticated endpoint that fetches arbitrary URLs is a proxy, a
scanner, and an amplifier pointed at third parties.

**Controls:** the S1 guard, unchanged and unrelaxed · IP rate limit 5/hour plus a global concurrency
cap · one URL pair per request, no lists · no shared cache with tenant crawls (so it cannot be used
as an oracle for what others monitor — [06 §4.2](06-crawler.md#42-the-tenant-question)) · short
wall-clock timeout · per-domain limits shared with the main crawler so the public tool cannot be
used to hammer a site the crawler is politely rate-limiting · circuit breaker that disables the
endpoint under abuse.

**Proof:** rate-limit integration tests; a load test in Phase 7 confirming the global cap holds.

---

## S4 — Credential and session compromise · **High**

**Controls:** Argon2id at OWASP baseline (19 MiB, t=2, p=1), tuned up if the host allows ·
**session tokens stored as SHA-256 hashes**, so a database dump is not a set of working sessions ·
`HttpOnly; Secure; SameSite=Lax` · rotation on privilege change · revocation is a database write,
not cookie deletion · password reset revokes all sessions · login rate limiting + lockout · uniform
response timing and wording so login and reset never reveal whether an email exists.

**Proof:** `crates/auth/tests/` — hash verification, session expiry/revocation, timing-uniformity
assertions on the negative paths.

---

## S5 — Stripe webhook forgery / replay · **High**

An unauthenticated endpoint that grants entitlements.

**Controls:** signature verification against `STRIPE_WEBHOOK_SECRET` on the **raw body** before any
parsing · timestamp tolerance to reject replays · `stripe_events (event_id PRIMARY KEY)` makes
processing idempotent (§39) · entitlements are derived from Stripe's state, never from request
parameters.

**Proof:** tests for bad signature → 400, replayed event → 200 with no second effect, out-of-order
delivery converging to the correct plan.

---

## S6 — CSRF · **Medium**

Cookie auth means a cross-site form post would otherwise carry credentials.

**Controls:** double-submit token bound to the session row · `SameSite=Lax` as defence in depth ·
enforced by middleware on every state-changing method, so a route cannot opt out by omission ·
`Origin`/`Referer` checked on state-changing requests.

**Proof:** middleware test asserting that every non-GET route rejects a missing/foreign token.

---

## S7 — Resource exhaustion · **Medium**

A 500 MB CSV, a 50 000-backlink bulk create, an unbounded export, a crawl of a page that streams
forever.

**Controls:** 1 MiB body limit globally / 50 MiB on upload only · bulk create capped at 1000 ·
imports chunked into bounded jobs so one tenant cannot monopolize a worker · crawler response cap
2 MiB with streaming abort · per-job wall-clock budget · plan-based quotas (§39) · `acquire_timeout`
on the pool so overload sheds rather than queues.

**Proof:** oversized-body tests; the oversized-response crawler fixture.

---

## S8 — XSS via crawled content · **Medium**

Anchor text, canonical URLs and error strings come from **attacker-controlled pages** and are
rendered back in our UI. This is the non-obvious one: the injection source is not our user, it is a
third-party publisher.

**Controls:** React escapes by default and we never use `dangerouslySetInnerHTML` for crawled
content · anchor text stored as text and length-capped · URLs validated as URLs and rendered via
`<a href>` with scheme allowlisting, never interpolated into `javascript:`-capable attributes ·
strict CSP (no `unsafe-inline`) on the web app · CSV export cells prefixed against formula injection
(`=`, `+`, `-`, `@`, tab, CR) so a malicious anchor text does not execute in the customer's Excel.

**Proof:** a fixture whose anchor text is `<img src=x onerror=alert(1)>` asserted to render inert;
a CSV export test asserting formula-injection prefixing.

---

## S9 — SQL injection · **Low**

**Controls:** SQLx parameterized queries throughout; `query!`/`query_as!` macros verify against the
schema at compile time. Dynamic filtering (§35) is built with a whitelist-driven builder — sort
columns and filter fields map through a fixed `enum`, never string concatenation of user input.

**Proof:** no `format!`-built SQL exists; a clippy/CI grep denies `query(&format!` patterns.

---

## S10 — Secret leakage in logs · **Medium**

**Controls:** secrets typed as `secrecy::SecretString` (no `Debug` output) · a tracing layer that
redacts known-sensitive field names · DSNs logged with credentials stripped · request/response
bodies never logged at `info` · `.env` git-ignored and `.env.example` valueless.

**Proof:** a test asserting that `format!("{:?}", config)` contains no secret material.

---

## S11 — Denial of service against publishers (**by us**) · **Medium**

Not a risk *to* us — a risk we *create*. A bug in the scheduler could point 50 concurrent workers at
one publisher and take their site down. That is an outage we caused for a third party, and the
reputational failure mode for a monitoring product.

**Controls:** per-registrable-domain concurrency cap and minimum interval, shared globally including
the public tool · `robots.txt` respected · `Retry-After` honoured · identifying user agent with a
policy URL · exponential backoff with jitter · the dedup design means growth in customers does not
mean growth in requests to a given page.

**Proof:** a concurrency test asserting that N jobs for one domain serialize to the configured
per-domain limit.

---

## S12 — Preview deployments touching production · **Medium**

**Controls:** `APP_ENV` gates in one place ([10 §1](10-environments.md#1-appenv-gates-dangerous-behaviour)) ·
preview Neon branches from sanitized `staging`, never `main` · Stripe test keys only · email
captured, not sent · scheduler tick rejected · a test that fails when a new dangerous capability is
added without a gate.

---

## Phase 7 audit checklist (§55)

- [ ] SSRF suite passes, including the redirect-to-private and DNS-rebinding cases
- [ ] Tenant isolation suite covers 100% of tenant-owned routes (enforced by the route table)
- [ ] `cargo audit` / `cargo deny` clean; `pnpm audit` clean
- [ ] Argon2 parameters re-benchmarked on production hardware
- [ ] CSP has no `unsafe-inline`; security headers verified in production responses
- [ ] Rate limits verified under load, not just in unit tests
- [ ] Stripe webhook replay and out-of-order tested against Stripe CLI
- [ ] Neon connection behaviour verified (open item R1)
- [ ] Backup/restore rehearsed end-to-end, not merely configured
- [ ] Logs reviewed for secret leakage on a real traffic sample
