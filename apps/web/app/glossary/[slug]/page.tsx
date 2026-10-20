import type { Metadata } from "next";
import { notFound } from "next/navigation";
import { marketingPage } from "../../marketing";
import { GLOSSARY, GLOSSARY_BY_SLUG, type GlossaryTerm } from "../glossary-data";

// Programmatic glossary pages: one static page per term from glossary-data.ts,
// built on the shared marketing shell. Prerendered at build time; unknown slugs
// 404 (dynamicParams = false). Content is authored and trusted — the term
// strings carry intentional inline HTML (<b>, <code>, <i>) rendered as-is.
export const dynamic = "force-static";
export const dynamicParams = false;

const BASE = "https://www.linkpulz.com";

export function generateStaticParams() {
  return GLOSSARY.map((t) => ({ slug: t.slug }));
}

export async function generateMetadata({
  params,
}: {
  params: Promise<{ slug: string }>;
}): Promise<Metadata> {
  const { slug } = await params;
  const t = GLOSSARY_BY_SLUG[slug];
  if (!t) return {};
  const path = `/glossary/${t.slug}`;
  return {
    title: { absolute: t.title },
    description: t.description,
    alternates: { canonical: path, languages: { en: path, "x-default": path } },
    robots: { index: true, follow: true, "max-image-preview": "large" },
    openGraph: {
      siteName: "LinkPulz",
      type: "article",
      title: t.title,
      description: t.description,
      url: path,
    },
  };
}

// Escape a value for safe embedding inside the JSON-LD template literal.
function j(s: string): string {
  return s.replace(/\\/g, "\\\\").replace(/"/g, '\\"');
}

function buildJsonLd(t: GlossaryTerm): string {
  const url = `${BASE}/glossary/${t.slug}`;
  return `{
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
      "@type": "WebPage",
      "@id": "${url}#webpage",
      "url": "${url}",
      "name": "${j(t.title)}",
      "description": "${j(t.description)}",
      "isPartOf": { "@id": "${BASE}/#website" },
      "breadcrumb": { "@id": "${url}#breadcrumb" },
      "mainEntity": { "@id": "${url}#term" },
      "inLanguage": "en"
    },
    {
      "@type": "BreadcrumbList",
      "@id": "${url}#breadcrumb",
      "itemListElement": [
        { "@type": "ListItem", "position": 1, "name": "Home", "item": "${BASE}/" },
        { "@type": "ListItem", "position": 2, "name": "Glossary", "item": "${BASE}/glossary" },
        { "@type": "ListItem", "position": 3, "name": "${j(t.term)}", "item": "${url}" }
      ]
    },
    {
      "@type": "DefinedTerm",
      "@id": "${url}#term",
      "name": "${j(t.term)}",
      "description": "${j(t.description)}",
      "url": "${url}",
      "inDefinedTermSet": {
        "@type": "DefinedTermSet",
        "@id": "${BASE}/glossary#set",
        "name": "LinkPulz Backlink Glossary",
        "url": "${BASE}/glossary"
      }
    }
  ]
}`;
}

function buildMain(t: GlossaryTerm): string {
  const sections = t.sections
    .map(
      (s) => `
  <div class="wrap" style="max-width:760px">
    <h2>${s.h}</h2>
    ${s.body.map((p) => `<p>${p}</p>`).join("\n    ")}
  </div>`,
    )
    .join("\n");

  const related = t.related
    .map((slug) => GLOSSARY_BY_SLUG[slug])
    .filter(Boolean)
    .map(
      (r) =>
        `<div class="card"><h3><a href="/glossary/${r.slug}">${r.term}</a></h3></div>`,
    )
    .join("\n      ");

  const clusterLine = t.cluster
    ? `<p class="micro" style="margin-top:14px">Related: <a href="${t.cluster.href}">${t.cluster.label}</a>.</p>`
    : "";

  return `
<div class="wrap">
  <nav class="crumbs" aria-label="Breadcrumb"><a href="/">Home</a><span>›</span><a href="/glossary">Glossary</a><span>›</span>${t.term}</nav>
</div>

<section class="hero" aria-labelledby="h1">
  <div class="wrap" style="max-width:760px">
    <span class="badge">Glossary</span>
    <h1 id="h1">${t.h1}</h1>
    ${t.definition.map((p) => `<p class="lead">${p}</p>`).join("\n    ")}
    ${clusterLine}
  </div>
</section>

${sections}

<section class="alt" aria-labelledby="rel-h">
  <div class="wrap">
    <div class="center"><h2 id="rel-h">Related terms</h2></div>
    <div class="cards">
      ${related}
    </div>
  </div>
</section>

<section class="final" aria-labelledby="final-h">
  <div class="wrap">
    <h2 id="final-h">Monitor the backlinks behind the theory</h2>
    <p class="lead">LinkPulz tracks every link you earn or buy and alerts you the moment one is lost, redirected, nofollowed or broken.</p>
    <div class="cta-row" style="justify-content:center"><a class="btn" href="/register">Start your free trial</a></div>
  </div>
</section>
`;
}

export default async function GlossaryTermPage({
  params,
}: {
  params: Promise<{ slug: string }>;
}) {
  const { slug } = await params;
  const t = GLOSSARY_BY_SLUG[slug];
  if (!t) notFound();
  return <div dangerouslySetInnerHTML={{ __html: marketingPage(buildJsonLd(t), buildMain(t)) }} />;
}
