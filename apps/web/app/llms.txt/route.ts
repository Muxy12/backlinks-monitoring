import { SITE_URL } from "@/lib/site";

// /llms.txt, the llmstxt.org convention: a concise, LLM-friendly map of the site
// so assistants can find the important pages without scraping HTML. Served as
// text/plain markdown and regenerated hourly with the current canonical origin.
//
// A fuller, single-fetch context file lives at /llms-full.txt (product overview,
// feature list, full pricing table and FAQ) and machine-readable pricing at
// /pricing.txt. Keep the three in sync when plans or positioning change.
export const revalidate = 3600;

export async function GET() {
  const body = `# LinkPulz

> LinkPulz is a backlink monitoring platform for SEO teams, link builders and
> agencies. It re-checks every backlink you build or earn on a schedule you set
> and alerts you the moment a link is lost, redirected, broken, nofollowed or
> noindexed, plus anchor/rel changes, warranty expiries and the link graph for
> you and your competitors. 10-day free trial, no credit card. Plans $29–$199/mo.

## Product
- [Home](${SITE_URL}/): what LinkPulz does, who it's for, and how monitoring works.
- [Pricing](${SITE_URL}/pricing): Starter $29, Pro $59, Business $99, Agency $199 per month; 10-day free trial, no credit card. Machine-readable at ${SITE_URL}/pricing.txt.
- [Sign in](${SITE_URL}/login)
- [Create an account](${SITE_URL}/register): start the free trial.

## Use cases
- [Lost backlinks](${SITE_URL}/lost-backlinks): detect when an earned or paid link disappears and get alerted.
- [Broken backlinks](${SITE_URL}/broken-backlinks): find links pointing to 4xx/5xx or redirected target URLs.
- [Verify a backlink](${SITE_URL}/verify-backlinks): confirm a page still links to your target, with rel/dofollow and HTTP status.
- [Backlink health check](${SITE_URL}/backlink-health-check): score the overall health of a backlink profile.

## Guides
- [Backlink management](${SITE_URL}/backlink-management): organise and monitor a backlink portfolio.
- [Backlink audit](${SITE_URL}/backlink-audit): review a link profile for lost, broken and changed links.
- [Backlink profile](${SITE_URL}/backlink-profile): understand a site's full backlink profile and anchor distribution.
- [Backlink reporting](${SITE_URL}/backlink-reporting): report on backlink status and changes to clients.

## Tools & docs
- [Free backlink checker](${SITE_URL}/tools/backlink-checker): check whether a page links to a target URL, with rel/dofollow and HTTP status. No sign-up.
- [Documentation](${SITE_URL}/docs): getting started, backlink monitoring, competitors, link tiers, discovery, the graph, and the API.

## Full context
- [llms-full.txt](${SITE_URL}/llms-full.txt): product overview, full feature and pricing detail, and FAQ in one file.

## Legal
- [Privacy Policy](${SITE_URL}/legal/privacy)
- [Terms of Service](${SITE_URL}/legal/terms)
- [Cookie Policy](${SITE_URL}/legal/cookies)
- [Legal notice](${SITE_URL}/legal/notice): operated by MeedWise (SASU), Paris, France.
`;

  return new Response(body, {
    headers: {
      "content-type": "text/plain; charset=utf-8",
      "cache-control": "public, max-age=0, s-maxage=3600, stale-while-revalidate=86400",
    },
  });
}
