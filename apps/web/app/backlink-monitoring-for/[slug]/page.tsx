import type { Metadata } from "next";
import { notFound } from "next/navigation";
import { marketingPage } from "../../marketing";
import { PERSONAS, PERSONA_BY_SLUG, type Persona } from "../persona-data";

// Programmatic persona pages: "backlink monitoring for [audience]". One static
// page per entry in persona-data.ts, on the shared marketing shell. Content
// reframes real product features per audience — no invented stats or features.
export const dynamic = "force-static";
export const dynamicParams = false;

const BASE = "https://www.linkpulz.com";

export function generateStaticParams() {
  return PERSONAS.map((p) => ({ slug: p.slug }));
}

export async function generateMetadata({
  params,
}: {
  params: Promise<{ slug: string }>;
}): Promise<Metadata> {
  const { slug } = await params;
  const p = PERSONA_BY_SLUG[slug];
  if (!p) return {};
  const path = `/backlink-monitoring-for/${p.slug}`;
  return {
    title: { absolute: p.title },
    description: p.description,
    alternates: { canonical: path, languages: { en: path, "x-default": path } },
    robots: { index: true, follow: true, "max-image-preview": "large" },
    openGraph: {
      siteName: "LinkPulz",
      type: "website",
      title: p.title,
      description: p.description,
      url: path,
    },
  };
}

function j(s: string): string {
  return s.replace(/\\/g, "\\\\").replace(/"/g, '\\"');
}

function buildJsonLd(p: Persona): string {
  const url = `${BASE}/backlink-monitoring-for/${p.slug}`;
  const faq = p.faq
    .map(
      (f) =>
        `        { "@type": "Question", "name": "${j(f.q)}", "acceptedAnswer": { "@type": "Answer", "text": "${j(
          f.a,
        )}" } }`,
    )
    .join(",\n");
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
      "name": "${j(p.title)}",
      "description": "${j(p.description)}",
      "isPartOf": { "@id": "${BASE}/#website" },
      "about": { "@id": "${BASE}/#software" },
      "breadcrumb": { "@id": "${url}#breadcrumb" },
      "inLanguage": "en"
    },
    {
      "@type": "BreadcrumbList",
      "@id": "${url}#breadcrumb",
      "itemListElement": [
        { "@type": "ListItem", "position": 1, "name": "Home", "item": "${BASE}/" },
        { "@type": "ListItem", "position": 2, "name": "Backlink monitoring for ${j(p.audience)}", "item": "${url}" }
      ]
    },
    {
      "@type": "SoftwareApplication",
      "@id": "${BASE}/#software",
      "name": "LinkPulz",
      "applicationCategory": "BusinessApplication",
      "applicationSubCategory": "SEO software",
      "operatingSystem": "Web",
      "description": "Backlink monitoring software that checks your earned and paid backlinks on a schedule and alerts you when a link is lost, redirected, nofollowed, noindexed or broken.",
      "url": "${BASE}/",
      "publisher": { "@id": "${BASE}/#organization" },
      "offers": { "@type": "Offer", "price": "0", "priceCurrency": "USD", "description": "10-day free trial, no credit card. Paid plans from $29/mo.", "url": "${BASE}/#pricing" }
    },
    {
      "@type": "FAQPage",
      "@id": "${url}#faq",
      "mainEntity": [
${faq}
      ]
    }
  ]
}`;
}

function buildMain(p: Persona): string {
  const pains = p.pains
    .map((b) => `<div class="card"><h3>${b.h}</h3><p>${b.p}</p></div>`)
    .join("\n      ");
  const how = p.how
    .map((b) => `<div class="card"><h3>${b.h}</h3><p>${b.p}</p></div>`)
    .join("\n      ");
  const faq = p.faq
    .map((f) => `<details><summary>${f.q}</summary><p>${f.a}</p></details>`)
    .join("\n      ");
  const related = p.related
    .map((slug) => PERSONA_BY_SLUG[slug])
    .filter(Boolean)
    .map(
      (r) =>
        `<div class="card"><h3><a href="/backlink-monitoring-for/${r.slug}">${r.audience}</a></h3></div>`,
    )
    .join("\n      ");
  const cluster = p.clusterLinks
    .map((c) => `<a href="${c.href}">${c.label}</a>`)
    .join(" · ");

  return `
<div class="wrap">
  <nav class="crumbs" aria-label="Breadcrumb"><a href="/">Home</a><span>›</span>Backlink monitoring for ${p.audience}</nav>
</div>

<section class="hero" aria-labelledby="h1">
  <div class="wrap" style="max-width:820px">
    <span class="badge">For ${p.audience}</span>
    <h1 id="h1">${p.h1}</h1>
    ${p.intro.map((t) => `<p class="lead">${t}</p>`).join("\n    ")}
    <div class="cta-row"><a class="btn" href="/register">Start your free trial</a><a class="btn ghost" href="/pricing">See pricing</a></div>
  </div>
</section>

<section class="alt" aria-labelledby="pains-h">
  <div class="wrap">
    <div class="center"><h2 id="pains-h">The challenge</h2></div>
    <div class="cards">
      ${pains}
    </div>
  </div>
</section>

<section aria-labelledby="how-h">
  <div class="wrap">
    <div class="center"><h2 id="how-h">How LinkPulz helps</h2></div>
    <div class="cards">
      ${how}
    </div>
    <p class="micro" style="margin-top:16px;text-align:center">Explore: ${cluster}.</p>
  </div>
</section>

<section class="alt" id="faq" aria-labelledby="faq-h">
  <div class="wrap">
    <div class="center"><h2 id="faq-h">FAQ</h2></div>
    <div class="faq">
      ${faq}
    </div>
  </div>
</section>

<section aria-labelledby="rel-h">
  <div class="wrap">
    <div class="center"><h2 id="rel-h">Backlink monitoring for other teams</h2></div>
    <div class="cards">
      ${related}
    </div>
  </div>
</section>

<section class="final" aria-labelledby="final-h">
  <div class="wrap">
    <h2 id="final-h">Monitor your backlinks with LinkPulz</h2>
    <p class="lead">Track every link you earn or buy and get alerted the moment one is lost, redirected, nofollowed or broken.</p>
    <div class="cta-row" style="justify-content:center"><a class="btn" href="/register">Start your free trial</a></div>
  </div>
</section>
`;
}

export default async function PersonaPage({
  params,
}: {
  params: Promise<{ slug: string }>;
}) {
  const { slug } = await params;
  const p = PERSONA_BY_SLUG[slug];
  if (!p) notFound();
  return <div dangerouslySetInnerHTML={{ __html: marketingPage(buildJsonLd(p), buildMain(p)) }} />;
}
