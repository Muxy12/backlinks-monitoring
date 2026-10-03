# 18 — SEO page pipeline (programmatic SEO)

Single source of truth for the marketing / SEO page build-out. Each page is a
Server Component on the shared marketing shell (`apps/web/app/marketing.ts`,
`marketingPage(jsonLd, main)`), injected via `dangerouslySetInnerHTML`, zero
client JS, LinkPulz dark+purple charte scoped under `#lp-home`.

See [13-roadmap.md](13-roadmap.md) for product scope and
[14-design-system.md](14-design-system.md) for the charte.

## Build conventions (every page)

- **URL**: lowercase, hyphenated subfolder, **no trailing slash** (Next default
  routing). Internal links and JSON-LD `@id`/`url`/breadcrumb `item` must use the
  no-slash form so they match the canonical — see the audit fix in commit
  `39b811c`.
- **Metadata**: `title.absolute` (≈50–58 chars, keyword-first, `| LinkPulz`
  suffix), unique `description` (≈150–160 chars), `alternates.canonical`,
  `robots: index/follow/max-image-preview:large`, `openGraph`.
- **JSON-LD graph**: `Organization` + `WebSite` + `WebPage` + `BreadcrumbList` +
  `SoftwareApplication` + `FAQPage` (+ `HowTo` for step-by-step guides). FAQ
  schema must mirror the on-page `<details>` FAQ verbatim.
- **Content**: breadcrumb → hero (with a charte demo visual) → unique body
  sections (alternate `.alt`) → on-page FAQ → "More tools" cross-links → final
  CTA. Primary CTA → `/register`.
- **Internal linking**: hub-and-spoke. Cross-link 3 relevant siblings; add the
  page to `app/sitemap.ts` and to the footer in `marketing.ts` (no orphans).
- **Honesty**: any product-feature claim not yet verified gets a
  `<span class="tc">TO CONFIRM: …</span>` marker.

## Legend

- **Status**: ✅ Built · 🟡 Queued · ⛔ Blocked
- **Playbook** (see `programmatic-seo` skill): Glossary · How-to/Guide ·
  Comparison · Curation/Directory · Conversion · Checklist/Template

## Built (1–10)

| # | URL | Title | Playbook | Status |
|---|-----|-------|----------|--------|
| 1 | `/` | Backlink Monitoring Tool: Lost Link Alerts | Persuade (home) | ✅ |
| 2 | `/verify-backlinks` | Verify Backlinks: Backlink Verification Tool | Tool/Glossary | ✅ |
| 3 | `/lost-backlinks` | Lost Backlinks: Find & Recover Lost Links | Glossary/How-to | ✅ |
| 4 | `/broken-backlinks` | Broken Backlinks: Find & Fix | Glossary/How-to | ✅ |
| 5 | `/backlink-health-check` | Backlink Health Check | Tool/Glossary | ✅ |
| 6 | `/backlink-management` | Backlink Management: Track & Monitor | Glossary/Guide | ✅ |
| 7 | `/backlink-audit` | Backlink Audit: How to Audit Your Backlinks | How-to | ✅ |
| 8 | `/backlink-profile` | Backlink Profile: What It Is & How to Analyze | Glossary/Guide | ✅ |
| 9 | `/backlink-reporting` | Backlink Reporting: Reports for Clients & Teams | Guide | ✅ |
| 10 | `/tools/backlink-checker` | Free Backlink Checker | Tool | ✅ |

## Queued (11–23)

| # | URL | Working title (provisional) | Playbook | Search intent / angle | Notes |
|---|-----|-----------------------------|----------|-----------------------|-------|
| 11 | `/bad-backlinks` | Bad Backlinks: How to Find & Handle Them | Glossary/How-to | What makes a link "bad", how to spot bad links in your profile, what to do | Tie to `/backlink-audit`. Watch cannibalization vs `/backlink-farms` (23) — bad = quality judgement, farms = a specific source type |
| 12 | `/backlink-exchange` | Backlink Exchange: Do Link Exchanges Still Work? | Glossary | What a link/backlink exchange is, SEO risk, when reciprocity is natural | ⚠️ Link-scheme adjacent — **cautionary stance** (Google link-spam policy), do not promote. Relates to 18 |
| 13 | `/backlink-finder` | Backlink Finder: Find Who Links to You | Tool | Find/discover backlinks pointing to a site | ⚠️ Cannibalization risk vs `/tools/backlink-checker` (10) and `/verify-backlinks` (2). Decide: discovery tool vs checker. May be a `/tools/*` page not a guide |
| 14 | `/301-redirect-backlinks` | 301 Redirects & Backlinks: Preserve Link Equity | How-to/Glossary | Using 301s to keep equity, redirected backlinks, chains/loops | Tie to `/broken-backlinks` (4). Explain redirect ≠ lost, and bad redirects |
| 15 | `/link-prospecting-tools` | Link Prospecting Tools: Find Link Opportunities | Curation/Directory | Roundup of tools/tactics to find link prospects | Top-of-funnel link building — slightly outside monitoring core. Curation page; keep honest, not just self-promo |
| 16 | `/alternatives/moz` | Moz Alternative: LinkPulz vs Moz for Link Monitoring | Comparison | "moz alternative" / "moz link explorer alternative" | 🔗 **Creates the `/alternatives/` hub** — needs an `/alternatives` index page too. Lets us re-link the "Alternatives" link removed from the home/footer. Comparison must be fair & factual (no fabricated feature claims) |
| 17 | `/guest-post-outreach` | Guest Post Outreach: A Practical Guide | How-to/Guide | Outreach process for guest posts & links | Pair with a future `/guest-post-link-monitoring` angle; emphasise monitoring the links you earn |
| 18 | `/reciprocal-links` | Reciprocal Links: Help or Hurt Your SEO? | Glossary | What reciprocal links are, when natural vs a scheme | ⚠️ Cautionary stance; relates to 12 |
| 19 | `/tier-2-links` | Tier 2 Links: What Tiered Link Building Is | Glossary | Tiered link building explained, risks | ⚠️ Grey-hat adjacent — explain neutrally with risk caveats, do not advocate |
| 20 | `/link-building-checklist` | Link Building Checklist (Free, Step by Step) | Checklist/Template | Actionable checklist; lead-magnet potential | `HowTo`/`ItemList` schema. Strong internal-link hub to 2–9, 11, 14, 17 |
| 21 | `/backlink-indexing` | Backlink Indexing: Get Your Links Indexed | How-to/Glossary | Why links aren't indexed, how to encourage indexing | Tie to `/verify-backlinks` (is the linking page indexable?) |
| 22 | `/how-many-backlinks` | How Many Backlinks Do You Need to Rank? | Glossary (question) | Answer the question honestly + caveats; high informational volume | No fake benchmarks. Quality > count framing; link to `/backlink-profile` (8) |
| 23 | `/backlink-farms` | Backlink Farms & Link Farms: What They Are | Glossary | What link farms are, why they're risky, spotting them in your profile | ⚠️ Cautionary; tie to `/backlink-audit` (7) and `/bad-backlinks` (11) |

## Cross-cutting flags

- **Editorial stance on link-scheme topics (12, 15, 18, 19, 23):** LinkPulz is a
  legitimate monitoring tool. Cover these topics educationally with a clear,
  Google-aligned cautionary POV (link-spam policy). **Do not promote schemes.**
  Angle every one back to "monitor your profile for risky/lost/changed links".
  This protects E-E-A-T and keeps the domain out of bad association.
- **Cannibalization to resolve before building:** 11 vs 23 (bad vs farms);
  13 vs 10 vs 2 (finder vs checker vs verify). Give each a distinct primary
  keyword and canonical intent, or merge.
- **`/alternatives/` hub (16):** building the Moz spoke requires an
  `/alternatives` index. Once it exists, restore the "Alternatives" links removed
  from the homepage and footer in commit `39b811c`.
- **Feature claims:** reporting/scheduling/white-label/tagging/import and
  check-frequency remain `TO CONFIRM` until verified against the app
  (see `notifications`, `billing`, `projects`, `clients` surfaces).
- **Prioritization (suggested):** informational volume & low scheme-risk first —
  22, 14, 21, 20, 11, 16 — then the cautionary/grey-hat set (12, 18, 19, 23) and
  the prospecting/outreach pair (15, 17); 13 after the finder-vs-checker decision.
