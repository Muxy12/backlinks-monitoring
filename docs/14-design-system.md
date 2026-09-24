# 14 — Design system & frontend

**Spec refs:** §5, §34, §35, §36, §48, §49, §50
**Source of truth:** the homepage the product owner supplied (`backlink.html`, 1735 lines,
self-contained, no external assets). Its CSS custom properties are the canonical palette; this
document lifts them into the app.

> **Branding note.** The supplied homepage is branded **LinkOps** throughout — nav, hero, FAQ body
> copy, footer and the copyright line. That is **draft branding**. The product is **Backlio**.
> The rebrand is part of the port (§4.1) and touches more than the logo: "LinkOps" appears inside
> FAQ answers and feature prose, not only in headers. A find-and-replace is not sufficient — each
> occurrence needs reading, because some sentences scan differently with a different name.

---

## 1. Tokens

Taken verbatim from the homepage `:root`. **Dark-first** — the marketing page and the dashboard
mockup inside it are both dark, so the app is dark by default rather than a light app bolted onto a
dark site.

```css
--bg:           #07090d    /* page */
--surface:      #0d1117    /* cards, table surface */
--surface-2:    #111720    /* raised: nav, popovers, table header */
--border:       rgba(255,255,255,.08)
--border-hover: rgba(255,255,255,.15)
--text:         #f7f8fa
--muted:        #8b949e    /* secondary text, labels */
--muted-2:      #66707d    /* tertiary, timestamps */
--accent:       #8b5cf6    /* violet — primary action */
--accent-2:     #a78bfa    /* hover / focus ring */
--green:        #36d399    /* live */
--red:          #fb7185    /* lost, errors */
--yellow:       #fbbf24    /* changed, nofollow, warranty warning */
--blue:         #60a5fa    /* informational, redirected */
--radius:       18px
--max:          1180px     /* marketing container */
```

Font: `Inter, ui-sans-serif, system-ui, -apple-system, "Segoe UI", sans-serif` — **no webfont
request**. The homepage loads zero external resources and we keep that property; a self-hosted Inter
subset is added only if the system stack proves visually unacceptable.

### 1.1 Status colour mapping

The ten backlink states ([08](08-backlink-state-machine.md)) map onto four semantic colours. This
mapping is defined **once**, in `apps/web/lib/status.ts`, and every table cell, badge, chart series
and dashboard counter reads it — so a status never renders green in one place and grey in another.

| State | Colour | Rationale |
|---|---|---|
| `live` | `--green` | |
| `lost` · `source_error` · `target_error` | `--red` | actionable failure |
| `changed` · `nofollow` · `redirected` | `--yellow` | actionable, not fatal |
| `blocked` · `unknown` | `--muted` | **deliberately grey, not red** — see below |
| `pending` | `--muted-2` | |

`blocked` and `unknown` are grey because they mean *"we could not verify this"*, not *"this is
broken."* Colouring them red would undo in the UI exactly what §24 and the state machine take care
to get right in the backend. They are counted in their own dashboard tile, never folded into "lost."

### 1.2 Tailwind wiring

The tokens become CSS variables on `:root` in `globals.css`, and `tailwind.config.ts` references
them rather than duplicating hex values:

```ts
colors: {
  bg: "var(--bg)",
  surface: { DEFAULT: "var(--surface)", 2: "var(--surface-2)" },
  accent:  { DEFAULT: "var(--accent)", 2: "var(--accent-2)" },
  status:  { live: "var(--green)", lost: "var(--red)", changed: "var(--yellow)", info: "var(--blue)" },
}
```

shadcn/ui is initialised against these variables, so its components inherit the palette instead of
shipping their own. One palette, one place to change it.

---

## 2. Density

§48 asks for *"clean, dense but readable, fast, professional, minimal animation."* The homepage
already demonstrates the target: generous marketing spacing, but the embedded dashboard mockup is
tight. The app follows the mockup, not the marketing page.

- Table rows **36px**, not the 52px shadcn default. An agency scanning 2 800 backlinks needs rows on
  screen, not air.
- `13px` table text, `14px` body, tabular numerals for every cost, count and status column so
  columns align vertically.
- Sticky table header and sticky first column (status + source) on horizontal scroll.
- **Minimal animation**, per §48 and per the source file, which has two `transition` rules and no
  `@keyframes`. Transitions are limited to hover/focus affordances at ≤150ms. No page transitions,
  no skeleton shimmer, no scroll animation.

## 3. Breakpoints

The homepage uses exactly two: `900px` and `600px`. The app adds one desktop-dense tier and keeps
the same two, so marketing and app agree:

| | |
|---|---|
| `< 600px` | single column; the backlink table becomes a card list |
| `600–900px` | sidebar collapses to icons |
| `900–1400px` | full layout |
| `> 1400px` | wider table, more columns visible without scroll |

---

## 4. Routes

### 4.1 Marketing — port, don't rewrite

`backlink.html` becomes the Next.js marketing route group. The content, copy, structure and CSS are
already right; the work is mechanical:

```
apps/web/app/(marketing)/
├── layout.tsx          nav + footer extracted from the source
├── page.tsx            hero · social proof · problem · features · how · comparison
│                       · quote · pricing · FAQ · final CTA
└── tools/
    ├── backlink-checker/page.tsx          §50 — already linked from the footer
    ├── dofollow-backlink-checker/page.tsx §50
    └── redirect-checker/page.tsx          §50
```

Rules for the port:

- Copy is **kept verbatim apart from the rebrand** (LinkOps → Backlio, see the branding note above).
  It is accurate about what the product does, and §51's honesty constraints are already respected —
  the FAQ describes Value at Risk as *"the recorded acquisition cost associated with backlinks that
  currently require attention… not guaranteed SEO or financial loss,"* which is exactly §33, and the
  Ahrefs question is answered honestly per §52. Do not let a rewrite quietly introduce a claim we
  cannot support.
- The hero dashboard mockup stays **static markup**. It is an illustration; wiring it to live data
  would make the marketing page depend on the API.
- Server Components throughout, no client JS. The source page needs none — the FAQ is markup, not an
  accordion — so the marketing route ships zero JavaScript.
- The footer already links `/tools/backlink-checker` and `/tools/dofollow-checker`. Those routes
  must exist before launch or be removed from the footer; a nav link to a 404 on the homepage is
  worse than a missing feature.

### 4.2 Application

```
apps/web/app/(app)/
├── layout.tsx                  sidebar per §48
├── dashboard/                  §34 — counters, then NEEDS ATTENTION
├── backlinks/                  §35 table + filters
│   └── [id]/                   §36 overview · commercial · monitoring · history
├── campaigns/  clients/  publishers/
├── alerts/  reports/  imports/
├── team/  billing/  settings/
└── onboarding/                 §49 — project → import → first results
```

Sidebar order is taken from the homepage's own dashboard mockup, which already matches §48:
**Overview · Backlinks · Campaigns · Clients · Publishers · Alerts · Reports**, with Imports, Team,
Billing and Settings below a divider.

---

## 5. Where business rules are *not*

The frontend renders and validates for UX. It decides nothing.

- Status is computed by the Rust state machine and delivered as a value. The frontend never derives
  "is this lost" from HTTP codes.
- Value at Risk arrives from `GET /dashboard` **already grouped by currency**
  ([03 §4.3](03-database-schema.md#43-attention-and-warranty)). The frontend does not sum across
  currencies — that would be a business rule, and a wrong one.
- Filtering, sorting and pagination are server-side (§35). The client holds a page, never the set.
- Client-side validation mirrors the server's rules for fast feedback; the server re-validates
  everything regardless.

## 6. Accessibility & performance

- The palette's muted greys on the dark surface need checking against WCAG AA — `--muted-2`
  (`#66707d`) on `--bg` (`#07090d`) is roughly 4.6:1, which passes for body text but is marginal at
  small sizes. Audited in Phase 1 with a contrast check in CI rather than by eye.
- Status is never conveyed by colour alone: every badge carries a text label, which matters for the
  red/green live-vs-lost distinction and for colour-blind users.
- Full keyboard navigation of the backlink table, including row actions.
- Marketing route: zero JS, no webfont, no external requests — it should reach a ~100 Lighthouse
  score, and it is the page the §50 SEO tools will link into.
