// Canonical public origin for the marketing site — used by metadataBase (so all
// `alternates.canonical` resolve to absolute URLs), the sitemap, robots.txt and
// llms.txt. Override per environment with NEXT_PUBLIC_APP_URL; the default is the
// production custom domain (NEVER the raw Vercel URL — that would split SEO
// signals and point Google at the wrong host).
export const SITE_URL = (
  process.env.NEXT_PUBLIC_APP_URL ?? "https://linkpulz.com"
).replace(/\/$/, "");

export const SITE_NAME = "LinkPulz";
export const SITE_TAGLINE = "Backlink monitoring for SEO teams";
