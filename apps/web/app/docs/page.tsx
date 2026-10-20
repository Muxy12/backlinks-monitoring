import type { Metadata } from "next";

export const dynamic = "force-static"; // public content → prerender + edge cache (pairs with the static marketing CSP in middleware.ts)
import { ProseShell, ProseFooter } from "../prose-chrome";

export const metadata: Metadata = {
  title: "Documentation",
  description:
    "How LinkPulz works: monitor backlinks, track lost links and changes, watch competitors, build link tiers, discover new backlinks, explore the graph, and use the API.",
  alternates: { canonical: "/docs", languages: { en: "/docs", "x-default": "/docs" } },
  robots: { index: true, follow: true, "max-image-preview": "large" },
  openGraph: {
    siteName: "LinkPulz",
    type: "article",
    title: "LinkPulz Documentation",
    description:
      "How LinkPulz works: monitor backlinks, track lost links and changes, watch competitors, build link tiers, discover new backlinks, explore the graph, and use the API.",
    url: "/docs",
  },
};

const JSONLD = `{
  "@context": "https://schema.org",
  "@graph": [
    {
      "@type": "Organization",
      "@id": "https://www.linkpulz.com/#organization",
      "name": "LinkPulz",
      "url": "https://www.linkpulz.com/",
      "logo": { "@type": "ImageObject", "url": "https://ye8jhaekbbcviws9.public.blob.vercel-storage.com/Logo_LinkPulz_en_n%C3%A9on_d%C3%A9grad%C3%A9.png" },
      "sameAs": ["https://www.linkedin.com/showcase/linkpulz", "https://x.com/linkpulz"]
    },
    {
      "@type": "WebSite",
      "@id": "https://www.linkpulz.com/#website",
      "url": "https://www.linkpulz.com/",
      "name": "LinkPulz",
      "inLanguage": "en",
      "publisher": { "@id": "https://www.linkpulz.com/#organization" }
    },
    {
      "@type": "TechArticle",
      "@id": "https://www.linkpulz.com/docs#article",
      "headline": "LinkPulz Documentation",
      "name": "Documentation",
      "description": "How LinkPulz works: monitor backlinks, track lost links and changes, watch competitors, build link tiers, discover new backlinks, explore the graph, and use the API.",
      "url": "https://www.linkpulz.com/docs",
      "inLanguage": "en",
      "isPartOf": { "@id": "https://www.linkpulz.com/#website" },
      "author": { "@id": "https://www.linkpulz.com/#organization" },
      "publisher": { "@id": "https://www.linkpulz.com/#organization" },
      "breadcrumb": { "@id": "https://www.linkpulz.com/docs#breadcrumb" }
    },
    {
      "@type": "BreadcrumbList",
      "@id": "https://www.linkpulz.com/docs#breadcrumb",
      "itemListElement": [
        { "@type": "ListItem", "position": 1, "name": "Home", "item": "https://www.linkpulz.com/" },
        { "@type": "ListItem", "position": 2, "name": "Documentation", "item": "https://www.linkpulz.com/docs" }
      ]
    }
  ]
}`;

export default function DocsPage() {
  return (
    <>
      <script type="application/ld+json" dangerouslySetInnerHTML={{ __html: JSONLD }} />
      <ProseShell>
      <h1>Documentation</h1>
      <p className="prose-updated">
        Everything you need to monitor your backlinks with LinkPulz.
      </p>

      <nav className="prose-toc" aria-label="Contents">
        <h2>Contents</h2>
        <ul>
          <li><a href="#getting-started">Getting started</a></li>
          <li><a href="#projects">Projects &amp; clients</a></li>
          <li><a href="#backlinks">Backlinks</a></li>
          <li><a href="#monitoring">Monitoring &amp; states</a></li>
          <li><a href="#competitors">Competitors</a></li>
          <li><a href="#tiers">Link tiers</a></li>
          <li><a href="#discovery">Discovery</a></li>
          <li><a href="#graph">The link graph</a></li>
          <li><a href="#import">Importing data</a></li>
          <li><a href="#api">API</a></li>
        </ul>
      </nav>

      <h2 id="getting-started">Getting started</h2>
      <ol>
        <li><a href="/register">Create an account</a> and your first workspace (organization).</li>
        <li>Add a <strong>project</strong> for the site whose backlinks you want to track.</li>
        <li>Add backlinks manually, or <a href="#import">import a CSV</a>.</li>
        <li>LinkPulz checks each link on a schedule and records what changes.</li>
      </ol>

      <h2 id="projects">Projects &amp; clients</h2>
      <p>
        A <strong>project</strong> represents one target site (a domain you want links pointing to).
        Agencies can group projects under <strong>clients</strong> to keep work organized and report
        per client. Every project, backlink and report is scoped to your workspace, data is never
        shared across organizations.
      </p>

      <h2 id="backlinks">Backlinks</h2>
      <p>A backlink is a link from a source page to your target URL. For each one LinkPulz tracks:</p>
      <ul>
        <li>Source URL &amp; domain, target URL &amp; domain, and anchor text.</li>
        <li><code>rel</code> attributes, <code>nofollow</code>, <code>sponsored</code>, <code>ugc</code>, and whether the link is dofollow.</li>
        <li>Link type (text or image, with alt text), HTTP status of source and target, and the final URL after redirects.</li>
        <li>First seen, last seen, last checked and (if lost) lost-at timestamps.</li>
        <li>Optional campaign, publisher, cost and warranty metadata.</li>
      </ul>

      <h2 id="monitoring">Monitoring &amp; states</h2>
      <p>
        Each check compares what we observe now to the last known state and appends an event. A link
        moves through clear states:
      </p>
      <ul>
        <li><strong>live</strong>, present and dofollow as expected.</li>
        <li><strong>nofollow / changed</strong>, still present but an attribute or anchor changed.</li>
        <li><strong>suspected_lost</strong>, missing on a single check; not yet confirmed lost.</li>
        <li><strong>lost</strong>, confirmed absent after repeated checks.</li>
        <li><strong>source_error / target_error / redirected</strong>, transient or structural issues.</li>
      </ul>
      <div className="prose-note">
        A single failed request never immediately marks a link as lost. Confirmation is asymmetric:
        a link is only declared lost after it is repeatedly absent, so a flaky fetch or a temporary
        outage does not create a false alarm.
      </div>
      <p>
        See the guides on <a href="/lost-backlinks">lost backlinks</a> and{" "}
        <a href="/broken-backlinks">broken backlinks</a> for how these states play out in practice, or{" "}
        <a href="/tools/backlink-checker">check a single backlink</a> right now.
      </p>

      <h2 id="competitors">Competitors</h2>
      <p>
        Add competitor domains to a project to track their referring domains alongside yours. The
        <strong> backlink gap</strong> shows domains that link to competitors but not to you -
        prioritized outreach targets.
      </p>

      <h2 id="tiers">Link tiers</h2>
      <p>
        Tier&nbsp;1 links point directly at your site. Tier&nbsp;2 links point at your tier&nbsp;1
        pages to strengthen them. LinkPulz tracks both and ties tier&nbsp;2 links to their parent, so
        you can see the full support structure behind a link.
      </p>

      <h2 id="discovery">Discovery</h2>
      <p>
        Finding <em>unknown</em> backlinks is a reverse-index problem a single crawler cannot solve at
        web scale. LinkPulz offers two complementary paths:
      </p>
      <ul>
        <li>
          <strong>Seed &amp; expand</strong>, import known referring domains (e.g. from a Google
          Search Console &quot;top linking sites&quot; export); our crawler visits each to turn
          domain-level hints into real source→target backlinks.
        </li>
        <li>
          <strong>External providers</strong>, connect a reverse-index provider (e.g. DataForSEO, or
          Bing Webmaster for your own verified site) to pull in backlinks you don&apos;t know about
          yet. Providers are optional and only run once their API keys are configured.
        </li>
      </ul>

      <h2 id="graph">The link graph</h2>
      <p>
        The <a href="/graph">graph view</a> renders your backlinks as an interactive 2D or 3D network
       , target, referring domains and pages, competitors and tier&nbsp;2 support, so you can see
        clusters and concentration at a glance.
      </p>

      <h2 id="import">Importing data</h2>
      <p>Bring existing data in via CSV. A typical row includes:</p>
      <pre><code>source_url,target_url,anchor_text,rel,campaign,cost,currency
https://example.com/post,https://yoursite.com/,Your Brand,dofollow,Q3 Outreach,150,USD</code></pre>
      <p>Duplicate source→target pairs are de-duplicated on import; errors are reported per row.</p>

      <h2 id="api">API</h2>
      <p>
        LinkPulz is built API-first. Endpoints are versioned under <code>/v1</code> and scoped to your
        organization, e.g.:
      </p>
      <pre><code>GET  /v1/me
GET  /v1/organizations/&#123;org&#125;/backlinks
POST /v1/organizations/&#123;org&#125;/backlinks
GET  /v1/organizations/&#123;org&#125;/dashboard</code></pre>
      <p>
        Requests authenticate with your session cookie and send an <code>X-CSRF-Token</code> header on
        writes. Full endpoint reference is coming soon.
      </p>

      <ProseFooter />
      </ProseShell>
    </>
  );
}
