# LinkPulz — Product documents

The six foundational product documents. They sit above the numbered engineering docs in `../` and
cross-link into them rather than duplicating detail.

| # | Document | What it answers |
|---|---|---|
| [01](01-prd.md) | **Product Requirement Document (PRD)** | What the product does, for whom, feature by feature; principles & non-goals. |
| [02](02-trd.md) | **Technical Requirement Document (TRD)** | How it's built: architecture, hosting, decisions, integrations, NFRs, CI. |
| [03](03-app-flow.md) | **App Flow** | How a user moves through it, page → endpoint, incl. the background monitoring loop. |
| [04](04-design-brief.md) | **Design Brief** | Brand, tokens, component patterns, CSP/a11y constraints, design principles. |
| [05](05-backend-schema.md) | **Backend / Data Schema** | The data model grouped by domain, invariants, migration ledger. |
| [06](06-implementation-plan.md) | **Implementation Plan** | Status (built/live), launch checklist, sequence, post-launch roadmap. |

Deeper engineering references: [`../00-index.md`](../00-index.md). Living operational state is tracked
in session memory (`launch-readiness`, `anchor-distribution`, `stripe-billing`, …).
