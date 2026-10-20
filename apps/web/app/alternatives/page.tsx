import type { Metadata } from "next";
import { marketingPage } from "../marketing";
import { ALTERNATIVES } from "./alternatives-data";

// The alternatives hub: lists every "[competitor] alternative" page.
export const dynamic = "force-static";

const BASE = "https://www.linkpulz.com";

export const metadata: Metadata = {
  title: { absolute: "Backlink Monitoring Tool Alternatives | LinkPulz" },
  description:
    "How LinkPulz compares to Ahrefs, Semrush, Monitor Backlinks, Moz and Majestic — a focused, affordable alternative for monitoring the backlinks you've earned.",
  alternates: { canonical: "/alternatives", languages: { en: "/alternatives", "x-default": "/alternatives" } },
  robots: { index: true, follow: true, "max-image-preview": "large" },
  openGraph: {
    siteName: "LinkPulz",
    type: "website",
    title: "Backlink Monitoring Tool Alternatives | LinkPulz",
    description:
      "How LinkPulz compares to Ahrefs, Semrush, Monitor Backlinks, Moz and Majestic for backlink monitoring.",
    url: "/alternatives",
  },
};

function j(s: string): string {
  return s.replace(/\\/g, "\\\\").replace(/"/g, '\\"');
}

const JSONLD = `{
  "@context": "https://schema.org",
  "@graph": [
    {
      "@type": "Organization",
      "@id": "${BASE}/#organization",
      "name": "LinkPulz",
      "url": "${BASE}/",
      "logo": { "@type": "ImageObject", "url": "https://ye8jhaekbbcviws9.public.blob.vercel-storage.com/Logo_LinkPulz_en_n%C3%A9on_d%C3%A9grad%C3%A9.png" },
      "sameAs": ["https://www.linkedin.com/showcase/linkpulz", "https://x.com/linkpulz"]
    },
    {
      "@type": "WebSite",
      "@id": "${BASE}/#website",
      "url": "${BASE}/",
      "name": "LinkPulz",
      "inLanguage": "en",
      "publisher": { "@id": "${BASE}/#organization" }
    },
    {
      "@type": "CollectionPage",
      "@id": "${BASE}/alternatives#webpage",
      "url": "${BASE}/alternatives",
      "name": "Backlink Monitoring Tool Alternatives | LinkPulz",
      "description": "How LinkPulz compares to other backlink and SEO tools for backlink monitoring.",
      "isPartOf": { "@id": "${BASE}/#website" },
      "breadcrumb": { "@id": "${BASE}/alternatives#breadcrumb" },
      "inLanguage": "en"
    },
    {
      "@type": "BreadcrumbList",
      "@id": "${BASE}/alternatives#breadcrumb",
      "itemListElement": [
        { "@type": "ListItem", "position": 1, "name": "Home", "item": "${BASE}/" },
        { "@type": "ListItem", "position": 2, "name": "Alternatives", "item": "${BASE}/alternatives" }
      ]
    },
    {
      "@type": "ItemList",
      "@id": "${BASE}/alternatives#list",
      "itemListElement": [
${ALTERNATIVES.map(
  (a, i) =>
    `        { "@type": "ListItem", "position": ${i + 1}, "name": "${j(
      a.competitor,
    )} alternative", "url": "${BASE}/alternatives/${a.slug}" }`,
).join(",\n")}
      ]
    }
  ]
}`;

const CARDS = ALTERNATIVES.map(
  (a) =>
    `      <div class="card"><h3><a href="/alternatives/${a.slug}">${a.competitor} alternative</a></h3><p>${a.category}</p></div>`,
).join("\n");

const MAIN = `
<div class="wrap">
  <nav class="crumbs" aria-label="Breadcrumb"><a href="/">Home</a><span>›</span>Alternatives</nav>
</div>

<section class="hero" aria-labelledby="h1">
  <div class="wrap" style="max-width:820px">
    <span class="badge">Compare</span>
    <h1 id="h1">LinkPulz vs. other backlink tools</h1>
    <p class="lead">Most SEO tools are broad suites or large link indexes. LinkPulz is focused on one job: monitoring the backlinks you've earned or bought, and alerting you the moment one changes. Here's how it compares to the tools people most often weigh it against.</p>
  </div>
</section>

<section aria-labelledby="alts-h">
  <div class="wrap">
    <div class="center"><h2 id="alts-h">Alternatives</h2></div>
    <div class="cards">
${CARDS}
    </div>
    <p class="micro center" style="margin-top:18px">Product names are trademarks of their respective owners. These pages compare at a category level based on publicly documented information; features and prices change, so check each vendor for current details.</p>
  </div>
</section>

<section class="final" aria-labelledby="final-h">
  <div class="wrap">
    <h2 id="final-h">Try the focused option</h2>
    <p class="lead">LinkPulz monitors every link you earn or buy and alerts you the moment one is lost, redirected, nofollowed or broken.</p>
    <div class="cta-row" style="justify-content:center"><a class="btn" href="/register">Start your free trial</a></div>
  </div>
</section>
`;

export default function AlternativesHubPage() {
  return <div dangerouslySetInnerHTML={{ __html: marketingPage(JSONLD, MAIN) }} />;
}
