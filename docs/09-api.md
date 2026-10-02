# 09 — API design

**Spec refs:** §19, §36, §40, §45. Axum REST, JSON, OpenAPI-documented.

---

## 1. Conventions

- Base path `/v1`. Versioned from day one — cheap now, impossible to retrofit.
- Auth via **`HttpOnly; Secure; SameSite=Lax` session cookie**, not a bearer token in JS-readable
  storage. `Lax` (not `Strict`) so that a link from a verification email lands logged in; CSRF is
  handled explicitly rather than leaned on `SameSite` alone.
- **CSRF**: double-submit. A `csrf_token` cookie (readable) must match the `X-CSRF-Token` header on
  every state-changing request. The token's hash is bound to the session row, so a token from
  another session is rejected.
- Errors are RFC 7807 `application/problem+json`:
  ```json
  { "type": "https://linkpulz.com/errors/validation",
    "title": "Validation failed", "status": 422,
    "detail": "source_url must be a valid http(s) URL",
    "errors": { "source_url": ["invalid_url"] },
    "request_id": "01J..." }
  ```
- **Cross-tenant access returns `404`, never `403`** — we do not confirm that another org's resource
  exists ([ADR-0004](adr/0004-tenant-isolation-in-rust.md)).
- Body limit 1 MiB globally; 50 MiB on the CSV upload route only.
- Every response carries `X-Request-Id`, which is also the tracing span id
  ([12](12-observability.md)).

### 1.1 Org scoping in the URL

Tenant-owned routes are **nested under the organization**:

```
/v1/organizations/{org_id}/projects
/v1/organizations/{org_id}/backlinks/{id}
```

rather than taking the org from an ambient "current organization" server-side session value. This is
deliberate: an agency user with five clients open in five tabs must not have one tab's action land
in another tab's organization. The `org_id` in the path is validated against membership on every
request, producing the `OrgScope` — it is an *assertion to be checked*, never a trusted input.

### 1.2 Pagination

Cursor-based, on all collections (§35 — server-side pagination and filtering):

```
GET /v1/organizations/{org}/backlinks?limit=50&cursor=eyJ...&status=lost&sort=-last_checked_at
→ { "data": [...], "next_cursor": "eyJ...", "total_estimate": 2481 }
```

Cursors encode the sort key plus id, so they are stable under concurrent inserts — offset pagination
silently skips and repeats rows when a background job is writing, which it always is here.
`total_estimate` is explicitly an estimate; an exact count over a filtered multi-million-row table
on every page load is not worth it.

---

## 2. Routes

### Auth (§19)
| Method | Path | Notes |
|---|---|---|
| `POST` | `/v1/auth/register` | Rate limited by IP + email. Always returns 202, whether or not the email exists. |
| `POST` | `/v1/auth/login` | Rate limited; account lockout after N failures. Uniform timing on failure. |
| `POST` | `/v1/auth/logout` | Revokes the session row, not just the cookie. |
| `POST` | `/v1/auth/verify-email` | Single-use token. |
| `POST` | `/v1/auth/password-reset/request` | Always 202 — never reveals whether the email is registered. |
| `POST` | `/v1/auth/password-reset/confirm` | Revokes **all** sessions for the user. |
| `GET`  | `/v1/me` | Current user + org memberships. |
| `PATCH`| `/v1/me` | Account settings. |
| `GET`  | `/v1/me/sessions` / `DELETE /v1/me/sessions/{id}` | Session revocation (§19). |

### Organizations & team
| Method | Path |
|---|---|
| `GET` / `POST` | `/v1/organizations` |
| `GET` / `PATCH` | `/v1/organizations/{org}` |
| `GET` | `/v1/organizations/{org}/members` |
| `PATCH` / `DELETE` | `/v1/organizations/{org}/members/{user_id}` |
| `POST` | `/v1/organizations/{org}/invitations` |
| `GET` / `DELETE` | `/v1/organizations/{org}/invitations{/id}` |
| `POST` | `/v1/invitations/accept` |

### Domain resources

`clients`, `projects`, `campaigns`, `publishers`, `tags` each get the standard five under
`/v1/organizations/{org}/…`: `GET` (list), `POST`, `GET /{id}`, `PATCH /{id}`, `DELETE /{id}`.

### Backlinks (§35, §36)
| Method | Path | Notes |
|---|---|---|
| `GET` | `…/backlinks` | Filters: `project_id`, `client_id`, `campaign_id`, `publisher_id`, `status`, `rel`, `tag`, `warranty`, `cost_min/max`, `created_after/before`, `q`. All server-side. |
| `POST` | `…/backlinks` | Single create. |
| `POST` | `…/backlinks/bulk` | Bulk create, capped at 1000 per call. |
| `GET` | `…/backlinks/{id}` | Overview + commercial + monitoring (§36). |
| `PATCH` / `DELETE` | `…/backlinks/{id}` | |
| `POST` | `…/backlinks/{id}/archive` | Soft. Clears `next_check_at`. |
| **`POST`** | **`…/backlinks/{id}/check`** | **Enqueues `check_backlink_now` (priority 100) and returns `202 { job_id }`. It does not crawl.** (§36, §57.5) |
| `GET` | `…/backlinks/{id}/events` | The §30 timeline. |
| `GET` | `…/backlinks/{id}/checks` | Raw check history. |
| `GET` | `…/backlinks/export` | CSV; streamed, async for large sets. |

### Dashboard, alerts, imports, billing
| Method | Path | Notes |
|---|---|---|
| `GET` | `…/dashboard` | §34 counters + needs-attention + tracked spend + value at risk. |
| `GET` | `…/notifications` , `POST …/notifications/{id}/read` | |
| `GET`/`PATCH` | `/v1/me/notification-preferences` | |
| `POST` | `…/imports` | Multipart upload → `202 { import_id }`. |
| `GET` | `…/imports/{id}` | Status + preview + validation errors. |
| `POST` | `…/imports/{id}/mapping` | Column mapping (§38). |
| `POST` | `…/imports/{id}/confirm` | Enqueues `process_import`. |
| `POST` | `/v1/billing/checkout` , `POST /v1/billing/portal` | Stripe. |
| `POST` | `/v1/billing/webhook` | **Signature-verified, idempotent, unauthenticated.** |

### Internal & public
| Method | Path | Auth |
|---|---|---|
| `POST` | `/internal/scheduler/tick` | Shared secret header, constant-time compared. |
| `GET` | `/health` , `/ready` | None. `/ready` checks the database. |
| `POST` | `/v1/public/backlink-check` | **None.** Aggressive IP rate limit + global circuit breaker. Same SSRF guard as the crawler (§26, §50). No result caching shared with tenant crawls. |

---

## 3. The rule that shapes every write route

§9 and §57.5: a user action **enqueues a job**. It never crawls inline.

`POST /backlinks/{id}/check` returns `202 Accepted` with a `job_id` that the client polls (or picks
up via the notifications feed). It does not block. The same applies to imports, exports over a size
threshold, and any bulk operation. The only endpoint that performs a live fetch in-request is the
public checker (§50), which is a single URL pair under a hard rate limit and a wall-clock timeout —
and it is deliberately the one endpoint with no shared cache, so it cannot become an oracle
([06 §4.2](06-crawler.md#42-the-tenant-question)).

---

## 4. Rate limiting

`tower_governor`, keyed differently per route class:

| Class | Key | Budget |
|---|---|---|
| Auth (login/register/reset) | IP **and** email | 5 / 15 min, strict |
| Public checker | IP | 5 / hour, plus a global concurrency cap |
| Authenticated reads | session | 300 / min |
| Authenticated writes | session | 60 / min |
| `check` enqueue | organization | tied to plan (§39) |
| Webhooks | — | not rate limited; protected by signature |

---

## 5. OpenAPI

Generated from the Axum route definitions with `utoipa`, served at `/v1/openapi.json` and rendered
at `/v1/docs`. CI regenerates the document and fails if it differs from the committed copy, so the
spec cannot silently drift from the implementation. `apps/web` generates its TypeScript client from
that same document, which means a breaking API change fails the frontend typecheck rather than
production.
