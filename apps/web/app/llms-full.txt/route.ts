import { SITE_URL } from "@/lib/site";

// /llms-full.txt — the fuller llmstxt.org file: everything an assistant needs
// about LinkPulz in a single clean-markdown fetch (overview, what it monitors,
// how it works, pricing, FAQ, page map). Mirrors the homepage and /pricing;
// keep in sync with /llms.txt, /pricing.txt and crates/billing/src/plans.rs.
export const revalidate = 3600;

export async function GET() {
  const body = `# LinkPulz — backlink monitoring software

> LinkPulz re-checks every backlink you've earned or paid for on a schedule you
> set and alerts you the moment one is lost, redirected, broken, turns nofollow
> or gets noindexed. It's built for SEO teams, link builders and agencies who
> need to protect the links they worked hard to earn. 10-day free trial, no
> credit card. Operated by MeedWise (SASU), Paris, France.

Canonical site: ${SITE_URL}/

## What it is

LinkPulz is a backlink monitoring platform. You add the backlinks you care about
(earned or paid), and LinkPulz checks each one on a fixed schedule — daily,
weekly, bi-weekly or monthly — then alerts you when anything changes. It also maps
the link graph for your own sites and competitors, tracks acquisition cost and
link warranties, and classifies anchor text distribution.

## What it monitors

A link isn't simply "there" or "gone" — LinkPulz tracks every way a link can
quietly stop working:

- Lost / removed links — the link is gone from the page it used to sit on.
- Broken links — the link is still there but its target (or the host page) returns a 4xx/5xx error.
- Redirected links — the target URL now 3xx-redirects somewhere else.
- Nofollow changes — a dofollow link became rel="nofollow" (or sponsored/ugc).
- Noindex / deindexed — the linking page is no longer indexable.
- Anchor and rel changes — the visible anchor text or link attributes changed.
- Warranty expiries — paid-link warranty periods coming to an end.

## How it works (three steps)

1. Add your backlinks — paste or CSV-import the links you've earned or bought.
2. LinkPulz checks them on the schedule you set and records every change.
3. You get an alert (email, and on higher plans webhook/Slack) that says exactly what changed — not just that something did.

## Free tool (no sign-up)

- Backlink checker (${SITE_URL}/tools/backlink-checker): paste a page and a target URL to check whether the link exists, whether it's live, its rel/dofollow status and the HTTP status code.

## Pricing

Every plan includes a 10-day free trial (no credit card) and checks every backlink
on a schedule you set. Prices in USD, VAT included where applicable. Annual billing
is 10× the monthly price (2 months free). Over-limit never stops monitoring or
deletes data. Full machine-readable version: ${SITE_URL}/pricing.txt

| Plan | Monthly | Annual (effective/mo) | Backlinks | Projects | Users |
|------|---------|-----------------------|-----------|----------|-------|
| Starter | $29 | $290 ($24) | 500 | 3 | 1 |
| Pro (most popular) | $59 | $590 ($49) | 2,500 | 10 | 3 |
| Business | $99 | $990 ($82) | 5,000 | 25 | 5 |
| Agency | $199 | $1,990 ($166) | 15,000 | 75 | 15 |

Monitoring and alerting work the same on every plan — plans differ by how much you
track and how large your team is. Pro adds webhook & Slack alerts, full change
history and team access; Business adds priority support and higher limits; Agency
adds dedicated support for client portfolios.

## FAQ

Q: Is there a free trial?
A: Yes — every plan starts with a 10-day free trial and no credit card is required.

Q: Do I need a credit card to start?
A: No. You start the trial with just an email; payment details are only needed to continue on a paid plan.

Q: What happens when my trial ends?
A: Nothing is deleted. Monitoring pauses and you choose a plan; all backlinks, projects and history resume as soon as you subscribe.

Q: How often do you check my backlinks?
A: On a schedule you set — daily, weekly, bi-weekly or monthly — on every plan.

Q: What happens if I reach my plan limit?
A: You keep monitoring everything already tracked; you just can't add new backlinks, projects or users until you upgrade. Nothing is deleted.

Q: Can I change plans or cancel later?
A: Yes — upgrade or downgrade anytime from billing settings, and cancel whenever you like (the plan stays active until the end of the paid period).

## Key pages

- Home: ${SITE_URL}/
- Pricing: ${SITE_URL}/pricing
- Documentation: ${SITE_URL}/docs
- Free backlink checker: ${SITE_URL}/tools/backlink-checker
- Lost backlinks: ${SITE_URL}/lost-backlinks
- Broken backlinks: ${SITE_URL}/broken-backlinks
- Verify a backlink: ${SITE_URL}/verify-backlinks
- Backlink health check: ${SITE_URL}/backlink-health-check
- Backlink management: ${SITE_URL}/backlink-management
- Backlink audit: ${SITE_URL}/backlink-audit
- Backlink profile: ${SITE_URL}/backlink-profile
- Backlink reporting: ${SITE_URL}/backlink-reporting
- Start free trial: ${SITE_URL}/register
- Sign in: ${SITE_URL}/login
- Legal: ${SITE_URL}/legal

## About

LinkPulz is operated by MeedWise (SASU), a company registered in Paris, France
(RCS Paris 109 701 466). Contact: contact@linkpulz.com. Legal notice:
${SITE_URL}/legal/notice
`;

  return new Response(body, {
    headers: {
      "content-type": "text/plain; charset=utf-8",
      "cache-control": "public, max-age=0, s-maxage=3600, stale-while-revalidate=86400",
    },
  });
}
