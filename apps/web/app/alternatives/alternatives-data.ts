// Alternative pages (programmatic SEO): "[competitor] alternative for backlink
// monitoring". HONESTY IS NON-NEGOTIABLE here: every statement about a competitor
// is a widely-documented, category-level fact (what kind of product it is, the
// metrics it is known for). No prices, no specific feature claims that could be
// wrong, no fabricated ratings/Review schema. Each page also states plainly when
// the competitor is the better choice, and carries a transparency note. LinkPulz
// is positioned on its real strengths only: focused backlink MONITORING.

export type AltBlock = { h: string; p: string };
export type AltFaq = { q: string; a: string };

export type Alternative = {
  slug: string;
  competitor: string; // "Ahrefs"
  h1: string;
  title: string;
  description: string;
  // One honest, verifiable sentence about what the competitor broadly is.
  category: string;
  intro: string[];
  // Where LinkPulz fits — real LinkPulz strengths only.
  linkpulzFor: AltBlock[];
  // When the competitor is the better choice — honest, builds trust.
  competitorFor: string[];
  faq: AltFaq[];
  related: string[];
  clusterLinks: { href: string; label: string }[];
};

// Shared honest framing of what LinkPulz is, reused across pages.
const LINKPULZ_FITS: AltBlock[] = [
  {
    h: "Monitoring links you already have",
    p: "LinkPulz watches the backlinks you have earned or bought and re-checks each on a schedule you set, alerting you the moment one is lost, redirected, nofollowed, deindexed or broken.",
  },
  {
    h: "Focused and affordable",
    p: "It does one job — backlink monitoring — rather than bundling a full SEO platform, with a 10-day free trial, no card, and paid plans from $29/mo based on how many links you monitor.",
  },
  {
    h: "Built around the monitoring workflow",
    p: "Tag links by campaign, publisher and cost, track warranties, import from CSV or Google Search Console, run competitor gap analysis, and export a link's full change history for a report.",
  },
];

const LINKPULZ_CLUSTER = [
  { href: "/", label: "Backlink monitoring" },
  { href: "/lost-backlinks", label: "Lost backlinks" },
  { href: "/pricing", label: "Pricing" },
];

export const ALTERNATIVES: Alternative[] = [
  {
    slug: "ahrefs",
    competitor: "Ahrefs",
    h1: "A focused Ahrefs alternative for backlink monitoring",
    title: "Ahrefs Alternative for Backlink Monitoring | LinkPulz",
    description:
      "Looking for a focused alternative to Ahrefs just for backlink monitoring? LinkPulz tracks the links you've earned and alerts you when they change. See how they compare.",
    category:
      "Ahrefs is a comprehensive SEO toolset, best known for its large backlink index alongside keyword research, site audit and rank tracking.",
    intro: [
      "Ahrefs is a broad, powerful SEO suite. If your main need is simply to keep watch on the backlinks you have already earned or paid for — and be told the moment one changes — a focused monitoring tool can be a simpler, cheaper fit than a full platform.",
      "LinkPulz is that focused tool. It is not an Ahrefs replacement for keyword research or link discovery; it is purpose-built for monitoring your own link portfolio.",
    ],
    linkpulzFor: LINKPULZ_FITS,
    competitorFor: [
      "You need a large backlink index to discover links you don't already know about.",
      "You want keyword research, content and rank-tracking tools in one place.",
      "You run full technical site audits as part of your workflow.",
    ],
    faq: [
      {
        q: "Is LinkPulz a full replacement for Ahrefs?",
        a: "No, and it doesn't try to be. Ahrefs is a broad SEO suite with a large backlink index, keyword research and more. LinkPulz is focused on monitoring the backlinks you already have and alerting you when they change.",
      },
      {
        q: "Why use LinkPulz instead of Ahrefs for monitoring?",
        a: "If monitoring your own links is the specific job you need done, a focused tool is usually simpler and more affordable than a full platform. LinkPulz re-checks each link on a schedule and alerts you by email, Slack or webhook.",
      },
      {
        q: "Can I import links I found in Ahrefs into LinkPulz?",
        a: "Yes. Export your backlinks to CSV and import them into LinkPulz to start monitoring them.",
      },
    ],
    related: ["semrush", "monitor-backlinks", "majestic"],
    clusterLinks: LINKPULZ_CLUSTER,
  },
  {
    slug: "semrush",
    competitor: "Semrush",
    h1: "A focused Semrush alternative for backlink monitoring",
    title: "Semrush Alternative for Backlink Monitoring | LinkPulz",
    description:
      "Want a focused alternative to Semrush just for watching your backlinks? LinkPulz monitors the links you've earned and alerts you when one changes. Compare the two.",
    category:
      "Semrush is a broad all-in-one SEO and digital marketing suite spanning keyword research, site audits, rank tracking, backlink analysis and more.",
    intro: [
      "Semrush is an all-in-one marketing suite with a wide feature set. If what you actually need is dependable monitoring of the backlinks you have built — not the whole platform — a dedicated tool can do that job with less overhead.",
      "LinkPulz focuses on exactly that: keeping watch on your link portfolio and telling you when something changes.",
    ],
    linkpulzFor: LINKPULZ_FITS,
    competitorFor: [
      "You want an all-in-one marketing suite beyond backlinks.",
      "You need keyword research, advertising and content tools together.",
      "You rely on a large index to discover new and competitor links at scale.",
    ],
    faq: [
      {
        q: "Does LinkPulz replace Semrush?",
        a: "Not for its full range. Semrush is a broad marketing suite. LinkPulz is a focused backlink monitor: it watches the links you already have and alerts you when they change.",
      },
      {
        q: "Can LinkPulz monitor links on a schedule like I'd expect from a suite?",
        a: "Yes. LinkPulz re-checks each link daily, weekly, bi-weekly or monthly on every plan, logs every check, and alerts you by email, Slack or webhook.",
      },
      {
        q: "Can I move my backlinks from Semrush to LinkPulz?",
        a: "Yes. Export them to CSV and import into LinkPulz to begin monitoring.",
      },
    ],
    related: ["ahrefs", "moz", "monitor-backlinks"],
    clusterLinks: LINKPULZ_CLUSTER,
  },
  {
    slug: "monitor-backlinks",
    competitor: "Monitor Backlinks",
    h1: "A Monitor Backlinks alternative for backlink monitoring",
    title: "Monitor Backlinks Alternative | LinkPulz",
    description:
      "Comparing Monitor Backlinks alternatives? LinkPulz is a focused backlink monitor that re-checks your links on a schedule and alerts you when one is lost or changed.",
    category:
      "Monitor Backlinks is a backlink monitoring and analysis tool focused on tracking your links and your competitors'.",
    intro: [
      "Monitor Backlinks sits in the same space as LinkPulz: dedicated backlink monitoring rather than a sprawling SEO suite. When two tools do the same core job, the right pick comes down to fit — the specific workflow, the alerting, the reporting and the pricing that match how you work.",
      "LinkPulz approaches monitoring with scheduled re-checks, email/Slack/webhook alerts, competitor gap analysis, cost and warranty tracking, and CSV/Google Search Console import.",
    ],
    linkpulzFor: LINKPULZ_FITS,
    competitorFor: [
      "You already use Monitor Backlinks and its workflow fits how you work.",
      "The right choice between two focused monitors is a matter of fit — compare both against your needs and current pricing.",
    ],
    faq: [
      {
        q: "How is LinkPulz different from Monitor Backlinks?",
        a: "Both are dedicated backlink monitors rather than full SEO suites. LinkPulz re-checks each link on a schedule, alerts you by email, Slack or webhook, and adds competitor gap analysis, cost/warranty tracking and CSV/Google Search Console import. The best fit depends on your workflow and current pricing — compare both.",
      },
      {
        q: "Can I try LinkPulz before switching?",
        a: "Yes. There's a 10-day free trial with no credit card, so you can monitor real links before deciding.",
      },
      {
        q: "Can I import my existing backlinks?",
        a: "Yes. Import from CSV or Google Search Console to start monitoring your links.",
      },
    ],
    related: ["ahrefs", "semrush", "majestic"],
    clusterLinks: LINKPULZ_CLUSTER,
  },
  {
    slug: "moz",
    competitor: "Moz",
    h1: "A focused Moz alternative for backlink monitoring",
    title: "Moz Alternative for Backlink Monitoring | LinkPulz",
    description:
      "Looking for a focused Moz alternative for backlink monitoring? LinkPulz watches the links you've earned and alerts you when one changes. See how the two compare.",
    category:
      "Moz is an SEO software suite known for metrics like Domain Authority (DA) and its Link Explorer backlink index, alongside keyword and rank-tracking tools.",
    intro: [
      "Moz is a well-known SEO suite with its own backlink index and metrics. If your specific need is to monitor the links you have already built, rather than research or score domains, a focused monitor is a leaner fit.",
      "LinkPulz concentrates on monitoring: it tracks your links and tells you when they change, without a composite authority score.",
    ],
    linkpulzFor: LINKPULZ_FITS,
    competitorFor: [
      "You rely on Domain Authority or Link Explorer for link research.",
      "You want keyword research and rank tracking in the same tool.",
      "You need a large index to discover links beyond the ones you already track.",
    ],
    faq: [
      {
        q: "Does LinkPulz provide a Domain Authority score?",
        a: "No. Domain Authority is Moz's own metric. LinkPulz focuses on observing your real backlinks and their changes rather than assigning a composite authority score.",
      },
      {
        q: "Why choose LinkPulz over Moz for monitoring?",
        a: "If monitoring your own links is the job, a focused tool is usually simpler and cheaper than a full suite. LinkPulz re-checks each link on a schedule and alerts you when one is lost, redirected, nofollowed or broken.",
      },
      {
        q: "Can I import links from Moz?",
        a: "Yes. Export your links to CSV and import them into LinkPulz to monitor them.",
      },
    ],
    related: ["ahrefs", "semrush", "majestic"],
    clusterLinks: LINKPULZ_CLUSTER,
  },
  {
    slug: "majestic",
    competitor: "Majestic",
    h1: "A focused Majestic alternative for backlink monitoring",
    title: "Majestic Alternative for Backlink Monitoring | LinkPulz",
    description:
      "Need a focused Majestic alternative for monitoring your backlinks? LinkPulz tracks the links you've earned and alerts you the moment one changes. Compare the two.",
    category:
      "Majestic is a backlink-focused platform known for its large link index and metrics like Trust Flow and Citation Flow.",
    intro: [
      "Majestic specializes in backlink data and index metrics. If your goal is to monitor the specific links you have earned or bought — and be alerted when they change — rather than explore a large index, a dedicated monitor is a closer fit.",
      "LinkPulz is built for monitoring: scheduled re-checks of your links, with alerts, rather than index-wide link metrics.",
    ],
    linkpulzFor: LINKPULZ_FITS,
    competitorFor: [
      "You want a large link index to explore and discover backlinks.",
      "You use Trust Flow or Citation Flow as part of your analysis.",
      "Index-wide link research matters more to you than monitoring your own links.",
    ],
    faq: [
      {
        q: "Does LinkPulz offer Trust Flow or Citation Flow?",
        a: "No. Those are Majestic's own metrics. LinkPulz focuses on monitoring your backlinks and reporting their changes, not on index-wide link scores.",
      },
      {
        q: "Why use LinkPulz instead of Majestic for monitoring?",
        a: "For the specific job of watching the links you already have, a focused monitor is simpler and more affordable. LinkPulz re-checks each link on a schedule and alerts you when one changes.",
      },
      {
        q: "Can I import backlinks into LinkPulz?",
        a: "Yes. Import from CSV or Google Search Console to start monitoring your links.",
      },
    ],
    related: ["ahrefs", "moz", "monitor-backlinks"],
    clusterLinks: LINKPULZ_CLUSTER,
  },
];

export const ALT_BY_SLUG: Record<string, Alternative> = Object.fromEntries(
  ALTERNATIVES.map((a) => [a.slug, a]),
);
