// Persona pages (programmatic SEO): "backlink monitoring for [audience]".
// Each entry renders one static page at /backlink-monitoring-for/<slug>. Content
// reframes LinkPulz's REAL features for a specific audience — no invented stats,
// no features the product doesn't have. Each page has a genuinely distinct angle
// (not a name-swap) so it earns its place.

export type PersonaBlock = { h: string; p: string };
export type PersonaFaq = { q: string; a: string };

export type Persona = {
  slug: string;
  audience: string; // "Agencies"
  h1: string;
  title: string;
  description: string;
  intro: string[];
  pains: PersonaBlock[]; // audience-specific challenges
  how: PersonaBlock[]; // how LinkPulz's real features help this audience
  faq: PersonaFaq[];
  related: string[]; // other persona slugs
  clusterLinks: { href: string; label: string }[];
};

export const PERSONAS: Persona[] = [
  {
    slug: "agencies",
    audience: "Agencies",
    h1: "Backlink monitoring for agencies",
    title: "Backlink Monitoring for SEO Agencies | LinkPulz",
    description:
      "Monitor every client's backlinks in one place. Group work by client, prove the links you built are still live, and export clean per-client reports.",
    intro: [
      "Agencies build links for many clients at once, and clients want proof those links stay live. Checking placements by hand across a dozen accounts does not scale, and a link that quietly disappears is both a ranking risk and an awkward conversation.",
      "LinkPulz is built for that reality: organize work by client, monitor every backlink on a schedule, and show clients the state of the links you earned for them.",
    ],
    pains: [
      {
        h: "Many clients, many portfolios",
        p: "Each client is a separate set of links, campaigns and target pages. Keeping them apart — and reporting per client — gets messy fast in a spreadsheet.",
      },
      {
        h: "Proving the work stayed live",
        p: "You earned or bought a placement months ago. Is it still there, still dofollow, still pointing at the right page? Clients ask, and \"let me check\" is not a great answer.",
      },
      {
        h: "Catching losses before the client does",
        p: "A removed or nofollowed link can slip by unnoticed until a ranking dip. You want to hear about it first.",
      },
    ],
    how: [
      {
        h: "Group projects under clients",
        p: "LinkPulz lets you group projects under clients, so each account's backlinks, campaigns and reports stay scoped to that client and nothing is shared across them.",
      },
      {
        h: "Scheduled checks + alerts",
        p: "Every link is re-checked on a schedule you set (daily to monthly) with email, Slack or webhook alerts, so a lost, redirected or nofollowed link reaches you the moment it is detected — not at the next audit.",
      },
      {
        h: "Clean exports for client reports",
        p: "Import links from CSV or Google Search Console, tag them by campaign or cost, and export every monitored link with its full change history to CSV for a client report whenever you need it.",
      },
    ],
    faq: [
      {
        q: "Can I keep each client's backlinks separate?",
        a: "Yes. Group projects under clients so every client's links, campaigns and reports are scoped to that client. Data is never shared across clients.",
      },
      {
        q: "Can I report backlink status per client?",
        a: "Yes. Export each client's monitored links with their full change history to CSV at any time, which you can drop straight into a client report.",
      },
      {
        q: "Will I be alerted when a client's link is lost?",
        a: "Yes. LinkPulz alerts you by email, Slack or webhook when a monitored link is lost, redirected, nofollowed or broken, with the source, target and what changed.",
      },
    ],
    related: ["consultants", "saas", "ecommerce"],
    clusterLinks: [
      { href: "/backlink-management", label: "Backlink management" },
      { href: "/backlink-reporting", label: "Backlink reporting" },
      { href: "/lost-backlinks", label: "Lost backlinks" },
    ],
  },
  {
    slug: "saas",
    audience: "SaaS companies",
    h1: "Backlink monitoring for SaaS companies",
    title: "Backlink Monitoring for SaaS Companies | LinkPulz",
    description:
      "Protect the backlinks pointing at your SaaS product, feature and comparison pages, watch competitor links, and get alerted the moment a link changes.",
    intro: [
      "For a SaaS company, backlinks to your product, feature and comparison pages are hard-won assets from content, partnerships and digital PR. They also sit on pages other people control, so they can change without warning.",
      "LinkPulz watches those links for you and surfaces where competitors are earning links you are not.",
    ],
    pains: [
      {
        h: "High-value links on pages you don't control",
        p: "A link from a review site, a partner's integration page or a roundup can move, get edited, or switch to nofollow after a site refresh.",
      },
      {
        h: "Competitors earning the links you want",
        p: "The sites linking to your competitors are often the best outreach targets for you — but finding them by hand is slow.",
      },
      {
        h: "Links spread across many target pages",
        p: "Product, pricing, feature and comparison pages each attract links; tracking them per page matters for understanding what's working.",
      },
    ],
    how: [
      {
        h: "Monitor every earned and paid link",
        p: "Add the links you earn or buy and LinkPulz re-checks each on a schedule, alerting you when one is lost, redirected, nofollowed, deindexed or broken.",
      },
      {
        h: "Competitor backlink gap analysis",
        p: "Add competitor domains and LinkPulz surfaces the referring domains that link to them but not yet to you — a ready-made shortlist for outreach.",
      },
      {
        h: "API-first and import-friendly",
        p: "LinkPulz is API-first and imports links from CSV or Google Search Console, so it fits an existing growth stack and workflow.",
      },
    ],
    faq: [
      {
        q: "Can I see which backlinks point to each product or feature page?",
        a: "Yes. Each backlink records its target URL, so you can review the links pointing at a specific product, pricing, feature or comparison page.",
      },
      {
        q: "Can LinkPulz show competitor backlinks?",
        a: "Yes. Competitor gap analysis surfaces referring domains that link to competing sites but not yet to yours, so you can turn them into outreach prospects.",
      },
      {
        q: "Does LinkPulz have an API?",
        a: "Yes. LinkPulz is API-first; endpoints are versioned under /v1 and scoped to your organization.",
      },
    ],
    related: ["ecommerce", "agencies", "affiliates"],
    clusterLinks: [
      { href: "/backlink-profile", label: "Backlink profile" },
      { href: "/", label: "Backlink monitoring" },
      { href: "/verify-backlinks", label: "Verify a backlink" },
    ],
  },
  {
    slug: "ecommerce",
    audience: "Ecommerce stores",
    h1: "Backlink monitoring for ecommerce stores",
    title: "Backlink Monitoring for Ecommerce Stores | LinkPulz",
    description:
      "Keep the backlinks to your category and product pages healthy. Catch links broken by catalog changes and get alerted when a hard-won link disappears.",
    intro: [
      "Ecommerce catalogs change constantly — products go out of stock, URLs change, categories get restructured. Each change can break a backlink that pointed at a page that no longer exists, wasting the authority it carried.",
      "LinkPulz catches both sides of the problem: links that disappear, and links that still exist but now point to an error.",
    ],
    pains: [
      {
        h: "Catalog changes break inbound links",
        p: "A discontinued product or a restructured category can turn a valuable backlink into a link to a 404, often without anyone noticing.",
      },
      {
        h: "Seasonal link-building campaigns",
        p: "Gift guides and seasonal roundups earn links that matter for a few months — you want to know they stayed live through the season.",
      },
      {
        h: "Many referring domains to track",
        p: "Links from blogs, deal sites and roundups add up; checking them by hand across a large catalog isn't realistic.",
      },
    ],
    how: [
      {
        h: "Catch broken backlinks",
        p: "LinkPulz flags links that still exist but point to a page returning an error, and links sitting on pages that now error — so a catalog change doesn't silently waste link equity.",
      },
      {
        h: "Scheduled monitoring with alerts",
        p: "Re-check every link on a schedule you set, with email, Slack or webhook alerts, so a link lost during a seasonal push reaches you while there's still time to act.",
      },
      {
        h: "Organize by campaign and cost",
        p: "Tag links by campaign, publisher and cost, and import or export via CSV, so a seasonal push stays organized and reportable.",
      },
    ],
    faq: [
      {
        q: "Can LinkPulz find backlinks broken by a catalog change?",
        a: "Yes. LinkPulz detects links that point to a page returning an error and links on pages that now error, which is exactly what a discontinued product or changed URL causes.",
      },
      {
        q: "Can I track a seasonal campaign's links?",
        a: "Yes. Tag links by campaign and export their change history to CSV, so you can see which placements stayed live through the season.",
      },
      {
        q: "How often are links checked?",
        a: "On a schedule you set: daily, weekly, bi-weekly or monthly, on every plan, with every check logged in the link's history.",
      },
    ],
    related: ["saas", "agencies", "affiliates"],
    clusterLinks: [
      { href: "/broken-backlinks", label: "Broken backlinks" },
      { href: "/lost-backlinks", label: "Lost backlinks" },
      { href: "/backlink-health-check", label: "Backlink health check" },
    ],
  },
  {
    slug: "consultants",
    audience: "SEO consultants & freelancers",
    h1: "Backlink monitoring for SEO consultants and freelancers",
    title: "Backlink Monitoring for SEO Consultants & Freelancers | LinkPulz",
    description:
      "A focused, affordable way to monitor the backlinks you build for clients, prove the work stayed live, and report without an enterprise toolset.",
    intro: [
      "Consultants and freelancers deliver link building as a service, often without the budget — or the need — for a heavyweight enterprise suite. What you do need is a reliable way to prove the links you built are still live, and to report it simply.",
      "LinkPulz is a focused backlink monitor, not a sprawling platform, which keeps it affordable and quick to use.",
    ],
    pains: [
      {
        h: "Proving your work holds up",
        p: "A client paid for placements; you need to show, months later, that they're still live and set up correctly.",
      },
      {
        h: "No budget for enterprise suites",
        p: "A full SEO platform is overkill — and overpriced — when your core need is monitoring the links you built.",
      },
      {
        h: "Reporting without busywork",
        p: "Manually re-checking each link before a client call is time you'd rather spend on the work itself.",
      },
    ],
    how: [
      {
        h: "Focused and affordable",
        p: "LinkPulz does one job well — monitoring the backlinks you've earned or bought — with a 10-day free trial, no card, and paid plans from $29/mo based on how many links you monitor.",
      },
      {
        h: "Automated checks, ready-to-share history",
        p: "Links are re-checked on your schedule and every change is logged, so you can export a link's history to CSV for a client instead of re-checking by hand.",
      },
      {
        h: "Alerts so you hear it first",
        p: "Email, Slack or webhook alerts mean a lost or changed link reaches you before your client notices a dip.",
      },
    ],
    faq: [
      {
        q: "Is LinkPulz affordable for a solo consultant?",
        a: "Yes. There's a 10-day free trial with no credit card, and paid plans start at $29/mo based on the number of backlinks you monitor. See the pricing page for current details.",
      },
      {
        q: "Can I prove to a client that links are still live?",
        a: "Yes. Every link is re-checked on a schedule and its change history is recorded, which you can export to CSV for a client report.",
      },
      {
        q: "Do I need a big SEO suite to use it?",
        a: "No. LinkPulz is a focused backlink monitor you can set up in minutes; it doesn't require or replace a full SEO platform.",
      },
    ],
    related: ["agencies", "affiliates", "saas"],
    clusterLinks: [
      { href: "/pricing", label: "Pricing" },
      { href: "/backlink-reporting", label: "Backlink reporting" },
      { href: "/verify-backlinks", label: "Verify a backlink" },
    ],
  },
  {
    slug: "affiliates",
    audience: "Affiliates & bloggers",
    h1: "Backlink monitoring for affiliates and bloggers",
    title: "Backlink Monitoring for Affiliates & Bloggers | LinkPulz",
    description:
      "Protect the backlinks that drive traffic to your money pages. Catch lost links, dofollow-to-nofollow changes and broken links before they cost you rankings.",
    intro: [
      "For affiliates and bloggers, backlinks to your best-earning pages are what keep them ranking and earning. When a link to a money page is removed or nofollowed, the drop in authority can show up as a drop in income.",
      "LinkPulz watches those links and tells you the moment one changes, so you can act while it's recent.",
    ],
    pains: [
      {
        h: "Links to money pages quietly disappear",
        p: "A removed or edited link to a top-earning review or comparison page can dent its rankings before you spot the cause.",
      },
      {
        h: "Dofollow links turning nofollow",
        p: "A publisher can switch a once-dofollow link to nofollow after the fact, changing its value while the link still looks present.",
      },
      {
        h: "Hard to watch every placement",
        p: "As your link profile grows, re-checking each placement by hand simply doesn't happen.",
      },
    ],
    how: [
      {
        h: "Track attribute changes, not just presence",
        p: "LinkPulz records rel attributes on every check and alerts you when a link switches dofollow to nofollow — not only when it disappears.",
      },
      {
        h: "Catch lost and broken links early",
        p: "When a link to a money page is removed, or points to a page that now errors, LinkPulz flags it with the likely cause so you can reach out while the change is fresh.",
      },
      {
        h: "Set it and get alerted",
        p: "Add your links, set a schedule, and get email, Slack or webhook alerts — no manual re-checking.",
      },
    ],
    faq: [
      {
        q: "Will LinkPulz tell me if a link changes from dofollow to nofollow?",
        a: "Yes. LinkPulz tracks rel attributes (nofollow, sponsored, ugc) on every check and alerts you when they change.",
      },
      {
        q: "Can I protect the backlinks to my best-earning pages?",
        a: "Yes. Add those links and LinkPulz monitors each on a schedule, alerting you the moment one is lost, redirected, nofollowed or broken.",
      },
      {
        q: "Does losing a backlink hurt my rankings?",
        a: "It can, depending on the link's value and your broader profile. Losing a backlink doesn't automatically cause a drop, but monitoring helps you spot the change and decide whether it needs follow-up.",
      },
    ],
    related: ["consultants", "saas", "ecommerce"],
    clusterLinks: [
      { href: "/lost-backlinks", label: "Lost backlinks" },
      { href: "/glossary/dofollow-vs-nofollow", label: "Dofollow vs nofollow" },
      { href: "/", label: "Backlink monitoring" },
    ],
  },
];

export const PERSONA_BY_SLUG: Record<string, Persona> = Object.fromEntries(
  PERSONAS.map((p) => [p.slug, p]),
);
