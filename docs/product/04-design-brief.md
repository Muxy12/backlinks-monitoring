# Design Brief — LinkPulz

The visual and interaction contract for the product. The full component/system reference is
[`../14-design-system.md`](../14-design-system.md); the source of truth for values is
`apps/web/app/tokens.css`. This brief states intent and rules.

---

## 1. Brand & tone

- **Name / domain:** LinkPulz · linkpulz.com (infra ids + cookie keys stay lowercase `backlio`).
- **Voice:** precise, honest, operator-grade. We describe what we observed; we never bluff a verdict
  or a "score". Calm confidence, no hype, no fake urgency.
- **Two surfaces, one brand:**
  - **App** — light, clean, data-dense: canvas `#f6f8fc`, deep-navy text `#0b1220`, electric-blue
    `--primary #2f6bff` with a violet `--violet #7c5cff` secondary accent.
  - **Marketing** — darker, gradient (blue→violet) hero treatment; same accents, more atmosphere.

## 2. Visual language (tokens)

| Token group | Values |
|---|---|
| Brand | `--primary #2f6bff`, `--violet #7c5cff`, `--cyan #06b6d4`, `--grad` (135° blue→violet) |
| Surfaces | `--bg`, `--surface`, `--surface-2/3`; borders `rgba(13,24,44,.09)` |
| Text | `--text #0b1220`, `--text-2`, `--muted`, `--muted-2` |
| **Status (semantic, load-bearing)** | `--live #16a34a`, `--lost #e11d48`, `--changed #d97706`, `--redirected`/`--violet`, `--nofollow #b45309`, `--broken #dc2626`, `--pending #64748b`, `--unknown #94a3b8` |
| Radius | `--r-xs 6` … `--r-2xl 18`; cards `--r-xl 14` |
| Spacing | 4-based scale `--s-1 … --s-12` |
| Shadow | `--shadow-xs … --shadow-pop` (soft navy) |
| Motion | `--t-fast 120ms` / `--t 160ms` / `--t-slow 220ms`, ease `cubic-bezier(.2,.8,.2,1)` |
| Type | `--font-sans` = Inter; `--font-mono` for URLs / ids / numbers |

**Status colors are a shared language.** The same token means the same thing in a table cell, a
distribution bar, a chart, and a badge. Never recolor a status ad hoc.

## 3. Layout & component patterns

- **Panels** (`.panel`): the base card — surface, 1px border, `--r-xl`, `--shadow-xs`, subtle
  fade-up on mount.
- **Distribution modules** (`.dist` + `.dist-bar`/`.dist-seg` + `.legend`): the reusable horizontal
  mix bar (status breakdown, follow ratio, link type, **anchor distribution**). New distributions
  reuse these primitives, not bespoke charts.
- **Detail drawer:** right-side drawer for a backlink's timeline/impact; reusable `.timeline`.
- **Tables** (`.bl-table`): dense, tabular-nums, muted uppercase headers.
- **Charts:** dependency-free inline **SVG** themed from tokens (`GrowthChart`, `BeforeAfterChart`,
  anchor bar). No charting library.
- **Toasts, coachmark tour, getting-started checklist** for onboarding.

## 4. Hard constraints

- **CSP:** strict nonce + `strict-dynamic`; **no unsafe-inline scripts**. `style-src` allows
  inline styles. Any new widget must work without inline/eval JS (that is why charts are hand-rolled
  SVG and the `.xlsx` parser uses no `eval`).
- **No CSS framework.** Tokens + plain CSS modules per route.
- **Light/dark & responsive:** colors are tokens so themes adapt; layouts work at phone width with a
  16px gutter and no horizontal page scroll.
- **Accessibility:** semantic roles on charts (`role="img"` + aria-label), focus-visible rings
  (`--primary-weak`), sufficient contrast on status colors against surfaces.

## 5. Logo & assets

- App logo in the login header and nav (Vercel Blob asset). Marketing hero uses the gradient brand
  treatment. **Pending operator asset:** 6 product screenshots for the `/home` image slots.

## 6. Design principles (apply to every new screen)

1. **Show the observation, label the uncertainty.** Pending/empty/unavailable states are first-class,
   with real dates, never a blank or a fake zero.
2. **One meaning per color.** Reuse the status token system.
3. **Reuse the primitive** (panel, dist bar, drawer, SVG chart) before inventing one.
4. **Dense but calm.** Tabular numbers, generous line-height, restrained motion.
