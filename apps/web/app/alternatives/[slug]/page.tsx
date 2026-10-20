import type { Metadata } from "next";
import { notFound } from "next/navigation";
import { marketingPage } from "../../marketing";
import { ALTERNATIVES, ALT_BY_SLUG, type Alternative } from "../alternatives-data";

// Programmatic "[competitor] alternative" pages. One static page per entry.
// Competitor statements are category-level and verifiable; no prices, no
// fabricated feature claims, and no Review/AggregateRating schema. Each page
// states when the competitor is the better choice and carries a transparency
// note. LinkPulz is positioned on its real strengths only.
export const dynamic = "force-static";
export const dynamicParams = false;

const BASE = "https://www.linkpulz.com";

export function generateStaticParams() {
  return ALTERNATIVES.map((a) => ({ slug: a.slug }));
}

export async function generateMetadata({
  params,
}: {
  params: Promise<{ slug: string }>;
}): Promise<Metadata> {
  const { slug } = await params;
  const a = ALT_BY_SLUG[slug];
  if (!a) return {};
  const path = `/alternatives/${a.slug}`;
  return {
    title: { absolute: a.title },
    description: a.description,
    alternates: { canonical: path, languages: { en: path, "x-default": path } },
    robots: { index: true, follow: true, "max-image-preview": "large" },
    openGraph: {
      siteName: "LinkPulz",
      type: "website",
      title: a.title,
      description: a.description,
      url: path,
    },
  };
}

function j(s: string): string {
  return s.replace(/\\/g, "\\\\").replace(/"/g, '\\"');
}

function buildJsonLd(a: Alternative): string {
  const url = `${BASE}/alternatives/${a.slug}`;
  const faq = a.faq
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
      "name": "${j(a.title)}",
      "description": "${j(a.description)}",
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
        { "@type": "ListItem", "position": 2, "name": "Alternatives", "item": "${BASE}/alternatives" },
        { "@type": "ListItem", "position": 3, "name": "${j(a.competitor)} alternative", "item": "${url}" }
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

function buildMain(a: Alternative): string {
  const fits = a.linkpulzFor
    .map((b) => `<div class="card"><h3>${b.h}</h3><p>${b.p}</p></div>`)
    .join("\n      ");
  const better = a.competitorFor.map((t) => `<li>${t}</li>`).join("\n        ");
  const faq = a.faq
    .map((f) => `<details><summary>${f.q}</summary><p>${f.a}</p></details>`)
    .join("\n      ");
  const related = a.related
    .map((slug) => ALT_BY_SLUG[slug])
    .filter(Boolean)
    .map(
      (r) =>
        `<div class="card"><h3><a href="/alternatives/${r.slug}">${r.competitor} alternative</a></h3></div>`,
    )
    .join("\n      ");
  const cluster = a.clusterLinks
    .map((c) => `<a href="${c.href}">${c.label}</a>`)
    .join(" · ");

  return `
<div class="wrap">
  <nav class="crumbs" aria-label="Breadcrumb"><a href="/">Home</a><span>›</span><a href="/alternatives">Alternatives</a><span>›</span>${a.competitor} alternative</nav>
</div>

<section class="hero" aria-labelledby="h1">
  <div class="wrap" style="max-width:820px">
    <span class="badge">${a.competitor} alternative</span>
    <h1 id="h1">${a.h1}</h1>
    <p class="lead">${a.category}</p>
    ${a.intro.map((t) => `<p class="lead">${t}</p>`).join("\n    ")}
    <div class="cta-row"><a class="btn" href="/register">Start your free trial</a><a class="btn ghost" href="/pricing">See pricing</a></div>
  </div>
</section>

<section class="alt" aria-labelledby="fits-h">
  <div class="wrap">
    <div class="center"><h2 id="fits-h">Where LinkPulz fits</h2></div>
    <div class="cards">
      ${fits}
    </div>
    <p class="micro" style="margin-top:16px;text-align:center">Explore: ${cluster}.</p>
  </div>
</section>

<section aria-labelledby="better-h">
  <div class="wrap" style="max-width:820px">
    <div class="center"><h2 id="better-h">When ${a.competitor} is the better choice</h2></div>
    <p class="lead">LinkPulz is focused, not all-in-one. ${a.competitor} is the better pick when:</p>
    <ul class="check">
        ${better}
    </ul>
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
    <div class="center"><h2 id="rel-h">Other alternatives</h2></div>
    <div class="cards">
      ${related}
    </div>
    <p class="micro center" style="margin-top:18px">LinkPulz is a backlink monitoring tool. ${a.competitor} and other product names are trademarks of their respective owners. This page compares at a category level based on publicly documented information; features and prices change, so check each vendor for current details.</p>
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

export default async function AlternativePage({
  params,
}: {
  params: Promise<{ slug: string }>;
}) {
  const { slug } = await params;
  const a = ALT_BY_SLUG[slug];
  if (!a) notFound();
  return <div dangerouslySetInnerHTML={{ __html: marketingPage(buildJsonLd(a), buildMain(a)) }} />;
}
