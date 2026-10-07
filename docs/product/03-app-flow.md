# App Flow — LinkPulz

How a user moves through the product, from first touch to daily use. Page routes are Next.js App
Router paths under `apps/web/app`; API paths are the backend endpoints each screen calls.

---

## 1. Acquisition → account

```
Marketing (/home, tool pages)  ──►  /register ──► verify email (Resend) ──► 10-day trial starts
        │                                              │
        └──► /waitlist (pre-launch mode)               └──► /login ⇄ /forgot ⇄ /reset
```

- **Marketing / SEO surface:** `/home`, `/pricing`, programmatic tool pages (`/verify-backlinks`,
  `/lost-backlinks`, `/broken-backlinks`, `/backlink-health-check`, `/backlink-audit`,
  `/backlink-management`, `/backlink-profile`, `/backlink-reporting`, `/tools/backlink-checker`),
  `/docs`, `/legal/*`. Free tool: `POST /v1/tools/backlink-check` (Turnstile-protected).
- **Pre-launch:** homepage is in waitlist mode (`/waitlist`, `POST /v1/waitlist`); flipping to the
  full site + open registration is a launch-day switch (see [`06-implementation-plan.md`](06-implementation-plan.md)).
- **Auth:** `/v1/auth/register|verify|login|logout|forgot|reset|resend-verification`. Team invites:
  `/invite` → `POST /v1/invitations/accept`.

## 2. First run (onboarding)

On first login the app auto-starts a **guided tour** (coachmark spotlight, CSP-safe, once per
browser) and shows a data-driven **Get started checklist** on `/dashboard`:

```
create project ──► add backlinks ──► set up alerts
   (auto-ticked from billing/usage + alert destinations)
```

## 3. Core loop (daily use)

```
/dashboard ──► /projects ──► /backlinks ──► (add / import) ──► monitoring ──► alerts
 command        1 project    inventory +      bulk-first +       scheduler       email on
 center         = 1 domain   anchor dist.     CSV/.xlsx          (tick cron)      change/loss
```

1. **Project** (`/projects`): one project = one monitored domain. `GET/POST .../projects`.
2. **Add backlinks** (`/backlinks`, bulk-first modal): single `POST .../backlinks`, bulk
   `POST .../backlinks/bulk`, or import via `/import` (`POST .../imports`; `.xlsx` converted
   client-side). "Monitor these links" checked by default.
3. **Inventory** (`/backlinks`): filter/search, classify, tag, warranty, per-project **anchor text
   distribution** (`GET .../projects/{id}/anchors`), status/follow/type distribution bars, CSV export
   (`.../backlinks/export`).
4. **Backlink detail drawer:** history (`.../events`), occurrences, Tier-2, check now
   (`.../check`), and Post-Link Impact (`.../impact`, gated).
5. **Alerts** (`/settings/notifications`): destinations (`.../alert-destinations`), immediate/digest,
   test send.

## 4. Growth flows

| Flow | Pages | Key endpoints |
|---|---|---|
| **Discovery** | `/import` | `.../discovery/seeds`, `.../discovery/run` |
| **Competitors & gap** | `/competitors` | `.../competitors`, `.../projects/{id}/backlink-gap`, `/backlink-graph` |
| **Link-building campaigns** (BETA) | `/campaigns`, `/campaigns/[id]` | `.../campaigns/*`, `.../prospects/*` (draft, convert) |
| **Post-Link Impact** (gated) | drawer + `/campaigns/[id]` + `/settings/integrations` | `.../impact`, `.../gsc/*`, `/v1/gsc/oauth/callback` |
| **Graph** | `/graph` | `.../backlink-graph` |
| **Reporting** | `/backlinks` export | `.../backlinks/export` (CSV) |

## 5. Account & operations

| Area | Page | Endpoints |
|---|---|---|
| Billing & plan | `/settings/billing` | `.../billing`, `/checkout`, `/portal`, `/usage`; `/v1/plans`, webhook |
| Team & roles | `/team` | `.../members`, `.../invitations` |
| Integrations | `/settings/integrations` | `.../gsc/*` |
| Clients (agency) | `/clients` | `.../clients` |
| Feedback | `/feedback` | `/v1/feedback` |
| Admin backoffice | `/admin` | `/v1/admin/*` (stats, orgs, users, billing, waitlist, ops, audit) — `is_platform_admin` only |

## 6. Background flow (no user)

```
GitHub Actions cron (10 min) ──► POST /internal/tick ──► scheduler enqueues due checks
   ──► worker claims jobs (FOR UPDATE SKIP LOCKED) ──► crawler fetches ──► state machine
   ──► backlink_events / backlink_checks ──► alerts (Resend) ──► daily snapshot rollup
```

See [`../07-job-queue.md`](../07-job-queue.md), [`../06-crawler.md`](../06-crawler.md),
[`../08-backlink-state-machine.md`](../08-backlink-state-machine.md).
