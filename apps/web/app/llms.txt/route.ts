import { SITE_URL } from "@/lib/site";

// /llms.txt — the llmstxt.org convention: a concise, LLM-friendly map of the site
// so assistants can find the important pages without scraping HTML. Served as
// text/plain markdown and regenerated hourly with the current canonical origin.
export const revalidate = 3600;

export async function GET() {
  const body = `# Backlio

> Backlio is a backlink monitoring platform for SEO teams and agencies. It tracks
> every backlink you build or earn, detects lost links, nofollow/anchor changes,
> redirects and warranty expiries, and maps your (and competitors') link graph.

## Docs
- [Documentation](${SITE_URL}/docs): getting started, backlink monitoring, competitors, tiers, discovery, the graph, and the API.

## Tools
- [Free backlink checker](${SITE_URL}/tools/backlink-checker): check whether a page links to a target URL, with rel/dofollow and HTTP status.

## Product
- [Home](${SITE_URL}/): overview and features.
- [Sign in](${SITE_URL}/login)
- [Create an account](${SITE_URL}/register)

## Legal
- [Privacy Policy](${SITE_URL}/legal/privacy)
- [Terms of Service](${SITE_URL}/legal/terms)
- [Cookie Policy](${SITE_URL}/legal/cookies)
`;

  return new Response(body, {
    headers: {
      "content-type": "text/plain; charset=utf-8",
      "cache-control": "public, max-age=0, s-maxage=3600, stale-while-revalidate=86400",
    },
  });
}
