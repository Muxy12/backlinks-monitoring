import { SITE_URL } from "@/lib/site";

// /pricing.txt, machine-readable pricing for AI agents that compare products
// programmatically before a human visits (ai-seo: opaque/JS-gated pricing gets
// filtered out of AI-mediated buying journeys). Plain markdown, no JS, no login.
//
// Prices, limits and the trial MIRROR the single source of truth,
// crates/billing/src/plans.rs (served live at GET /v1/plans), and match the
// public /pricing page. When a price or limit changes there, update both.
// Annual = 10× monthly (2 months free); the "/mo" annual figure is the yearly
// total ÷ 12, rounded.
export const revalidate = 3600;

export async function GET() {
  const body = `# Pricing, LinkPulz

LinkPulz is backlink monitoring software for SEO teams, link builders and agencies.
Every plan includes a 10-day free trial with no credit card required, and re-checks
every backlink on a schedule you set (daily, weekly, bi-weekly or monthly).

Currency: USD. VAT included where applicable. Billing: monthly or annual (annual is
billed once a year at the price of 10 months, 2 months free). Cancel anytime.
Going over a plan limit never stops monitoring or deletes data, you just can't add
new backlinks, projects or users until you upgrade.

## Starter
- Price: $29/month | $290/year ($24/mo effective)
- For: website owners and small SEO projects
- Limits: 500 backlinks monitored, 3 projects, 1 user
- Features: checks on a schedule you set; lost & removed link alerts; broken & redirected link detection; nofollow/noindex change alerts; CSV import; email alerts

## Pro (most popular)
- Price: $59/month | $590/year ($49/mo effective)
- For: SEO professionals and link builders
- Limits: 2,500 backlinks monitored, 10 projects, 3 users
- Features: everything in Starter, plus webhook & Slack alerts; full backlink change history; team access

## Business
- Price: $99/month | $990/year ($82/mo effective)
- For: SEO teams managing multiple websites
- Limits: 5,000 backlinks monitored, 25 projects, 5 users
- Features: everything in Pro, plus priority email support; higher monitoring limits

## Agency
- Price: $199/month | $1,990/year ($166/mo effective)
- For: agencies managing backlinks across many clients
- Limits: 15,000 backlinks monitored, 75 projects, 15 users
- Features: everything in Business, plus dedicated priority support; built for client portfolios

## Links
- Pricing page: ${SITE_URL}/pricing
- Start free trial: ${SITE_URL}/register
- Live plan data (JSON API): ${SITE_URL}/api/v1/plans
`;

  return new Response(body, {
    headers: {
      "content-type": "text/plain; charset=utf-8",
      "cache-control": "public, max-age=0, s-maxage=3600, stale-while-revalidate=86400",
    },
  });
}
