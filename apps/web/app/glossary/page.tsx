import type { Metadata } from "next";
import { marketingPage } from "../marketing";
import { GLOSSARY } from "./glossary-data";

// The glossary hub: lists every term and acts as the DefinedTermSet the term
// pages belong to. Static, on the shared marketing shell.
export const dynamic = "force-static";

const BASE = "https://www.linkpulz.com";

export const metadata: Metadata = {
  title: { absolute: "Backlink Glossary: Key SEO Link Terms Explained | LinkPulz" },
  description:
    "A plain-English glossary of backlink and link-building terms: backlinks, dofollow vs nofollow, referring domains, anchor text, link equity and more.",
  alternates: { canonical: "/glossary", languages: { en: "/glossary", "x-default": "/glossary" } },
  robots: { index: true, follow: true, "max-image-preview": "large" },
  openGraph: {
    siteName: "LinkPulz",
    type: "website",
    title: "Backlink Glossary: Key SEO Link Terms Explained | LinkPulz",
    description:
      "A plain-English glossary of backlink and link-building terms: backlinks, dofollow vs nofollow, referring domains, anchor text, link equity and more.",
    url: "/glossary",
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
      "@id": "${BASE}/glossary#webpage",
      "url": "${BASE}/glossary",
      "name": "Backlink Glossary: Key SEO Link Terms Explained | LinkPulz",
      "description": "A plain-English glossary of backlink and link-building terms.",
      "isPartOf": { "@id": "${BASE}/#website" },
      "breadcrumb": { "@id": "${BASE}/glossary#breadcrumb" },
      "mainEntity": { "@id": "${BASE}/glossary#set" },
      "inLanguage": "en"
    },
    {
      "@type": "BreadcrumbList",
      "@id": "${BASE}/glossary#breadcrumb",
      "itemListElement": [
        { "@type": "ListItem", "position": 1, "name": "Home", "item": "${BASE}/" },
        { "@type": "ListItem", "position": 2, "name": "Glossary", "item": "${BASE}/glossary" }
      ]
    },
    {
      "@type": "DefinedTermSet",
      "@id": "${BASE}/glossary#set",
      "name": "LinkPulz Backlink Glossary",
      "url": "${BASE}/glossary",
      "hasDefinedTerm": [
${GLOSSARY.map(
  (t) =>
    `        { "@type": "DefinedTerm", "@id": "${BASE}/glossary/${t.slug}#term", "name": "${j(
      t.term,
    )}", "url": "${BASE}/glossary/${t.slug}" }`,
).join(",\n")}
      ]
    }
  ]
}`;

const CARDS = GLOSSARY.map(
  (t) =>
    `      <div class="card"><h3><a href="/glossary/${t.slug}">${t.term}</a></h3><p>${t.description}</p></div>`,
).join("\n");

const MAIN = `
<div class="wrap">
  <nav class="crumbs" aria-label="Breadcrumb"><a href="/">Home</a><span>›</span>Glossary</nav>
</div>

<section class="hero" aria-labelledby="h1">
  <div class="wrap" style="max-width:760px">
    <span class="badge">Glossary</span>
    <h1 id="h1">Backlink glossary</h1>
    <p class="lead">Plain-English definitions of the backlink and link-building terms that come up when you monitor your links. Short, factual, and linked to the tools and guides that put each one into practice.</p>
  </div>
</section>

<section aria-labelledby="terms-h">
  <div class="wrap">
    <div class="center"><h2 id="terms-h">Terms</h2></div>
    <div class="cards">
${CARDS}
    </div>
  </div>
</section>

<section class="final" aria-labelledby="final-h">
  <div class="wrap">
    <h2 id="final-h">From definitions to live monitoring</h2>
    <p class="lead">LinkPulz tracks every backlink you earn or buy and alerts you the moment one is lost, redirected, nofollowed or broken.</p>
    <div class="cta-row" style="justify-content:center"><a class="btn" href="/register">Start your free trial</a></div>
  </div>
</section>
`;

export default function GlossaryHubPage() {
  return <div dangerouslySetInnerHTML={{ __html: marketingPage(JSONLD, MAIN) }} />;
}
