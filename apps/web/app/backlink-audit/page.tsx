import type { Metadata } from "next";
import { marketingPage } from "../marketing";

// Backlink Audit — SEO guide/tool page on the shared marketing shell. Unique
// content targeting "backlink audit" / "how to do a backlink audit". Intent: a
// point-in-time review that finds problems (lost, broken, nofollow, toxic),
// distinct from ongoing management and the profile-analysis page. TO CONFIRM
// markers flag feature claims to verify before launch.
export const metadata: Metadata = {
  title: { absolute: "Backlink Audit: How to Audit Your Backlinks | LinkPulz" },
  description:
    "A backlink audit, step by step: find lost, broken, nofollowed and risky links in your profile, fix what you can, and set up monitoring so new problems never pile up.",
  alternates: { canonical: "/backlink-audit" },
  robots: { index: true, follow: true, "max-image-preview": "large" },
  openGraph: {
    type: "website",
    title: "Backlink Audit: How to Audit Your Backlinks | LinkPulz",
    description:
      "Run a backlink audit step by step: find lost, broken, nofollowed and risky links, fix what you can, and monitor the rest.",
    url: "/backlink-audit",
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
      "logo": { "@type": "ImageObject", "url": "https://ye8jhaekbbcviws9.public.blob.vercel-storage.com/logo-linkpulz.webp" },
      "sameAs": ["https://www.linkedin.com/company/linkpulz", "https://x.com/linkpulz"]
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
      "@type": "WebPage",
      "@id": "https://www.linkpulz.com/backlink-audit#webpage",
      "url": "https://www.linkpulz.com/backlink-audit",
      "name": "Backlink Audit: How to Audit Your Backlinks | LinkPulz",
      "description": "A backlink audit, step by step: find lost, broken, nofollowed and risky links in your profile, fix what you can, and set up monitoring so new problems never pile up.",
      "isPartOf": { "@id": "https://www.linkpulz.com/#website" },
      "about": { "@id": "https://www.linkpulz.com/#software" },
      "breadcrumb": { "@id": "https://www.linkpulz.com/backlink-audit#breadcrumb" },
      "inLanguage": "en"
    },
    {
      "@type": "BreadcrumbList",
      "@id": "https://www.linkpulz.com/backlink-audit#breadcrumb",
      "itemListElement": [
        { "@type": "ListItem", "position": 1, "name": "Home", "item": "https://www.linkpulz.com/" },
        { "@type": "ListItem", "position": 2, "name": "Backlink Audit", "item": "https://www.linkpulz.com/backlink-audit" }
      ]
    },
    {
      "@type": "HowTo",
      "@id": "https://www.linkpulz.com/backlink-audit#howto",
      "name": "How to do a backlink audit",
      "description": "A step-by-step backlink audit: gather your links, check each one, flag the problems, act on them, and set up ongoing monitoring.",
      "step": [
        { "@type": "HowToStep", "position": 1, "name": "Gather every backlink", "text": "Pull your links together from past reports, your own records and an SEO backlink source so you are auditing one complete list." },
        { "@type": "HowToStep", "position": 2, "name": "Check each link's live state", "text": "Re-fetch each linking page and record whether your link is present, where it points, its anchor text and whether it is dofollow." },
        { "@type": "HowToStep", "position": 3, "name": "Flag the problems", "text": "Mark links that are lost, broken, redirected, switched to nofollow, on a noindex page, or that sit on low-quality or spammy sites." },
        { "@type": "HowToStep", "position": 4, "name": "Act on what you found", "text": "Recover lost links, fix or replace broken targets, and decide what to do about risky links." },
        { "@type": "HowToStep", "position": 5, "name": "Set up monitoring", "text": "Put the surviving links under monitoring so the next audit does not start from scratch and new problems are caught as they happen." }
      ]
    },
    {
      "@type": "SoftwareApplication",
      "@id": "https://www.linkpulz.com/#software",
      "name": "LinkPulz",
      "applicationCategory": "BusinessApplication",
      "applicationSubCategory": "SEO software",
      "operatingSystem": "Web",
      "description": "Backlink monitoring software that checks your earned and paid backlinks on a schedule and alerts you when a link is lost, redirected, nofollowed, noindexed or broken.",
      "url": "https://www.linkpulz.com/",
      "publisher": { "@id": "https://www.linkpulz.com/#organization" },
      "offers": { "@type": "Offer", "price": "0", "priceCurrency": "USD", "description": "Free trial. TO CONFIRM.", "url": "https://www.linkpulz.com/#pricing" }
    },
    {
      "@type": "FAQPage",
      "@id": "https://www.linkpulz.com/backlink-audit#faq",
      "mainEntity": [
        { "@type": "Question", "name": "What is a backlink audit?", "acceptedAnswer": { "@type": "Answer", "text": "A backlink audit is a point-in-time review of the links pointing to your site. You gather every backlink, check the live state of each one, and flag the problems, lost, broken, nofollowed, noindexed, redirected or risky links, so you can act on them." } },
        { "@type": "Question", "name": "How often should I audit my backlinks?", "acceptedAnswer": { "@type": "Answer", "text": "A full audit once or twice a year is a sensible baseline, plus one after any big change such as a site migration. Between audits, ongoing monitoring catches new problems as they happen so they do not pile up." } },
        { "@type": "Question", "name": "What problems does a backlink audit find?", "acceptedAnswer": { "@type": "Answer", "text": "Lost links that were removed, broken links that point to errors, links switched from dofollow to nofollow, links on pages that went noindex, redirected links, and links on low-quality or spammy sites." } },
        { "@type": "Question", "name": "What are toxic or spammy backlinks?", "acceptedAnswer": { "@type": "Answer", "text": "They are links from low-quality, irrelevant or manipulative sites. Most sites can safely ignore them, as search engines discount low-quality links on their own. Assessing and, in rare cases, disavowing them is a separate judgement call from the lost, broken and changed links monitoring handles." } },
        { "@type": "Question", "name": "Do I need a tool to audit backlinks?", "acceptedAnswer": { "@type": "Answer", "text": "You can audit a small set by hand, but checking the live state of dozens or hundreds of links one by one does not scale, and the result is out of date the moment you finish. A tool checks them for you and keeps the result current." } },
        { "@type": "Question", "name": "How does LinkPulz help with a backlink audit?", "acceptedAnswer": { "@type": "Answer", "text": "LinkPulz checks the live state of each link you add, present or lost, dofollow or nofollow, live or broken, and then keeps checking on a schedule, so your audit stays true instead of going stale. [TO CONFIRM: import, check frequency and alert channels]" } }
      ]
    }
  ]
}`;

const MAIN = `
<div class="wrap">
  <nav class="crumbs" aria-label="Breadcrumb"><a href="/">Home</a><span>›</span>Backlink Audit</nav>
</div>

<!-- HERO -->
<section class="hero" aria-labelledby="h1">
  <div class="wrap grid">
    <div>
      <span class="badge">Backlink audit</span>
      <h1 id="h1">Backlink Audit: Find the Problems in Your Link Profile</h1>
      <p class="lead">A backlink audit is a point-in-time review of every link pointing to your site. Gather them, check each one, and flag the problems, lost, broken, nofollowed and risky links, so you can fix what matters and stop the rest slipping through.</p>
      <div class="cta-row">
        <a class="btn" href="/register">Audit my backlinks</a>
        <a class="btn ghost" href="#steps">The 5 steps</a>
      </div>
      <p class="micro">No credit card required <span class="tc">TO CONFIRM</span></p>
    </div>
    <div class="lost-demo" role="img" aria-label="Example audit result: links flagged as live, nofollow, broken and lost">
      <p class="micro verify-label">Audit result</p>
      <div class="lost-row"><small>OK</small><span class="url">partner.com/resources</span><span class="status live">● LIVE</span></div>
      <div class="lost-row"><small>Flag</small><span class="url">blog.example.com/old</span><span class="status lost">● NOFOLLOW</span></div>
      <div class="lost-row"><small>Flag</small><span class="url">directory.net/listing</span><span class="status lost">● 404</span></div>
      <div class="lost-row"><small>Flag</small><span class="url">news.example.com/piece</span><span class="status lost">● LOST</span></div>
      <div class="lost-alert"><b>3 issues found</b><br>1 lost · 1 broken · 1 nofollowed.</div>
    </div>
  </div>
</section>

<!-- WHAT IT IS -->
<section class="alt" aria-labelledby="what-h">
  <div class="wrap">
    <div class="prose">
      <h2 id="what-h">What is a backlink audit?</h2>
      <p>A backlink audit is a structured review of the links pointing to your site, carried out at a single point in time. You collect every backlink you can find, check the live state of each one, and flag the ones that need attention.</p>
      <p>It answers a simple question: of all the links I think I have, which are actually live, which have quietly changed, and which are gone? An audit is the snapshot; <a href="/backlink-management">backlink management</a> is the ongoing process that keeps that snapshot true between reviews.</p>
    </div>
  </div>
</section>

<!-- WHEN -->
<section aria-labelledby="when-h">
  <div class="wrap">
    <div class="prose">
      <h2 id="when-h">When to run one</h2>
      <ul class="check">
        <li><b>On a regular cadence.</b> Once or twice a year keeps surprises small.</li>
        <li><b>After a site migration.</b> New URLs break links fast; audit as soon as the dust settles.</li>
        <li><b>After a ranking drop.</b> Lost or broken links are one thing to rule out.</li>
        <li><b>Before a big report.</b> Never present numbers you have not re-checked.</li>
        <li><b>When you inherit a site or client.</b> You cannot manage what you have never catalogued.</li>
      </ul>
    </div>
  </div>
</section>

<!-- STEPS -->
<section class="alt" id="steps" aria-labelledby="steps-h">
  <div class="wrap">
    <div class="prose">
      <h2 id="steps-h">How to do a backlink audit, step by step</h2>
      <ol>
        <li><b>Gather every backlink.</b> Pull them together from past reports, your own records and an SEO backlink source, so you audit one complete list rather than a partial one.</li>
        <li><b>Check each link's live state.</b> Re-fetch each linking page and record whether your link is present, where it points, its anchor text, and whether it is dofollow.</li>
        <li><b>Flag the problems.</b> Mark links that are lost, broken, redirected, switched to nofollow, on a noindex page, or sitting on low-quality or spammy sites.</li>
        <li><b>Act on what you found.</b> Recover lost links, fix or replace broken targets, and make a call on anything risky.</li>
        <li><b>Set up monitoring.</b> Put the surviving links under ongoing monitoring so your next audit does not start from zero.</li>
      </ol>
    </div>
  </div>
</section>

<!-- WHAT TO FLAG -->
<section aria-labelledby="flag-h">
  <div class="wrap">
    <div class="prose">
      <h2 id="flag-h">What to flag in the audit</h2>
      <p>Not every flagged link is the same problem, and each needs a different response:</p>
      <ul class="check">
        <li><b>Lost</b>, the link has been removed from the page. See <a href="/lost-backlinks">lost backlinks</a>.</li>
        <li><b>Broken</b>, the link points to an error, or sits on a page that errors. See <a href="/broken-backlinks">broken backlinks</a>.</li>
        <li><b>Nofollowed or noindexed</b>, the link is present but has stopped passing value.</li>
        <li><b>Redirected</b>, the link still resolves, but no longer lands where you intended.</li>
        <li><b>Risky</b>, the link sits on a low-quality, irrelevant or spammy site.</li>
      </ul>
      <p class="micro" style="margin-top:12px">A note on risky links: most sites can safely ignore low-quality links, as search engines discount them on their own. Assessing and, in rare cases, disavowing them is a separate judgement call from the lost, broken and changed links monitoring handles for you.</p>
    </div>
  </div>
</section>

<!-- AUDIT VS MONITORING -->
<section class="alt" aria-labelledby="vs-h">
  <div class="wrap">
    <div class="prose">
      <h2 id="vs-h">Audit once, or monitor always?</h2>
      <p>An audit and ongoing monitoring solve the same problem on different timescales, you want both.</p>
    </div>
    <div class="tablewrap">
      <table>
        <thead><tr><th></th><th>One-off audit</th><th>Ongoing monitoring</th></tr></thead>
        <tbody>
          <tr><th>Timing</th><td>A snapshot, on the day you run it</td><td>Continuous, on a schedule</td></tr>
          <tr><th>Finds new problems</th><td>Only at the next audit</td><td>As they happen</td></tr>
          <tr><th>Effort</th><td>High, and repeated each time</td><td>Set up once</td></tr>
          <tr><th>Best for</th><td>A deep clean of the whole profile</td><td>Keeping it clean afterwards</td></tr>
        </tbody>
      </table>
    </div>
    <div class="prose" style="margin-top:24px">
      <p>LinkPulz checks the live state of each link you add and then keeps checking on a schedule, so your audit stays true instead of going stale. <span class="tc">TO CONFIRM: import, check frequency and alert channels</span></p>
    </div>
  </div>
</section>

<!-- FAQ -->
<section aria-labelledby="faq-h">
  <div class="wrap">
    <div class="center"><h2 id="faq-h">Backlink audit: frequently asked questions</h2></div>
    <div class="faq">
      <details><summary>What is a backlink audit?</summary><p>A backlink audit is a point-in-time review of the links pointing to your site. You gather every backlink, check the live state of each one, and flag the problems, lost, broken, nofollowed, noindexed, redirected or risky links, so you can act on them.</p></details>
      <details><summary>How often should I audit my backlinks?</summary><p>A full audit once or twice a year is a sensible baseline, plus one after any big change such as a site migration. Between audits, ongoing monitoring catches new problems as they happen so they do not pile up.</p></details>
      <details><summary>What problems does a backlink audit find?</summary><p>Lost links that were removed, broken links that point to errors, links switched from dofollow to nofollow, links on pages that went noindex, redirected links, and links on low-quality or spammy sites.</p></details>
      <details><summary>What are toxic or spammy backlinks?</summary><p>They are links from low-quality, irrelevant or manipulative sites. Most sites can safely ignore them, as search engines discount low-quality links on their own. Assessing and, in rare cases, disavowing them is a separate judgement call from the lost, broken and changed links monitoring handles.</p></details>
      <details><summary>Do I need a tool to audit backlinks?</summary><p>You can audit a small set by hand, but checking the live state of dozens or hundreds of links one by one does not scale, and the result is out of date the moment you finish. A tool checks them for you and keeps the result current.</p></details>
      <details><summary>How does LinkPulz help with a backlink audit?</summary><p>LinkPulz checks the live state of each link you add, present or lost, dofollow or nofollow, live or broken, and then keeps checking on a schedule, so your audit stays true instead of going stale. <span class="tc">TO CONFIRM: import, check frequency and alert channels</span></p></details>
    </div>
  </div>
</section>

<!-- MORE TOOLS -->
<section class="alt" aria-labelledby="more-h">
  <div class="wrap">
    <div class="center"><h2 id="more-h">More backlink tools and guides</h2></div>
    <div class="cards">
      <div class="card"><h3><a href="/backlink-health-check">Backlink Health Check</a></h3><p>See the overall state of your backlink portfolio at a glance.</p></div>
      <div class="card"><h3><a href="/backlink-profile">Backlink Profile</a></h3><p>Understand the full picture: dofollow ratio, anchors and referring domains.</p></div>
      <div class="card"><h3><a href="/backlink-management">Backlink Management</a></h3><p>Keep every link in one place and monitor it between audits.</p></div>
    </div>
  </div>
</section>

<!-- FINAL CTA -->
<section class="final" aria-labelledby="final-h">
  <div class="wrap">
    <h2 id="final-h">Audit once. Then never let it go stale.</h2>
    <p class="lead">Run the audit, fix what matters, and put the survivors under monitoring with LinkPulz, so your next review starts from a profile that's already clean.</p>
    <div class="cta-row" style="justify-content:center"><a class="btn" href="/register">Audit my backlinks</a></div>
  </div>
</section>
`;

export default function BacklinkAuditPage() {
  return <div dangerouslySetInnerHTML={{ __html: marketingPage(JSONLD, MAIN) }} />;
}
