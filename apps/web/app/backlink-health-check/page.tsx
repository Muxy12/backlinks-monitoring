import type { Metadata } from "next";
import { marketingPage } from "../marketing";

// Backlink Health Check — SEO tool/guide page on the shared marketing shell.
// Unique content targeting "backlink health check". The dashboard figures are
// illustrative; TO CONFIRM markers flag claims to verify before launch (how the
// score is calculated, history, check frequency).
export const metadata: Metadata = {
  title: { absolute: "Backlink Health Check: Audit Your Link Profile | LinkPulz" },
  description:
    "Run a backlink health check: one score for your whole link profile, every problem grouped, tracked over time. See what to fix first and keep your links healthy.",
  alternates: { canonical: "/backlink-health-check", languages: { en: "/backlink-health-check", "x-default": "/backlink-health-check" } },
  robots: { index: true, follow: true, "max-image-preview": "large" },
  openGraph: {
    type: "website",
    title: "Backlink Health Check: Audit Your Link Profile | LinkPulz",
    description:
      "One health score for your whole link profile, every problem grouped and tracked over time. See what to fix first.",
    url: "/backlink-health-check",
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
      "@id": "https://www.linkpulz.com/backlink-health-check#webpage",
      "url": "https://www.linkpulz.com/backlink-health-check",
      "name": "Backlink Health Check: Audit Your Link Profile | LinkPulz",
      "description": "Run a backlink health check: one score for your whole link profile, every problem grouped, tracked over time. See what to fix first and keep your links healthy.",
      "isPartOf": { "@id": "https://www.linkpulz.com/#website" },
      "about": { "@id": "https://www.linkpulz.com/#software" },
      "breadcrumb": { "@id": "https://www.linkpulz.com/backlink-health-check#breadcrumb" },
      "inLanguage": "en"
    },
    {
      "@type": "BreadcrumbList",
      "@id": "https://www.linkpulz.com/backlink-health-check#breadcrumb",
      "itemListElement": [
        { "@type": "ListItem", "position": 1, "name": "Home", "item": "https://www.linkpulz.com/" },
        { "@type": "ListItem", "position": 2, "name": "Backlink Health Check", "item": "https://www.linkpulz.com/backlink-health-check" }
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
      "offers": { "@type": "Offer", "price": "0", "priceCurrency": "USD", "description": "10-day free trial, no credit card. Paid plans from $29/mo.", "url": "https://www.linkpulz.com/#pricing" }
    },
    {
      "@type": "FAQPage",
      "@id": "https://www.linkpulz.com/backlink-health-check#faq",
      "mainEntity": [
        { "@type": "Question", "name": "What is a backlink health check?", "acceptedAnswer": { "@type": "Answer", "text": "A backlink health check reviews the current state of every link pointing to your site, which are live, lost, broken, redirected, nofollow or changed, and summarises it so you can see what needs attention first." } },
        { "@type": "Question", "name": "How is a backlink health score calculated?", "acceptedAnswer": { "@type": "Answer", "text": "A health score summarises the share of your monitored links that are healthy versus those with a problem." } },
        { "@type": "Question", "name": "How do I run a backlink health check?", "acceptedAnswer": { "@type": "Answer", "text": "Add the links you want to track to LinkPulz. It checks each linking page on a schedule, groups the results by status, and shows what changed, so a health check becomes something that runs continuously rather than once." } },
        { "@type": "Question", "name": "What does a healthy backlink profile look like?", "acceptedAnswer": { "@type": "Answer", "text": "Most links live, few lost or broken, attributes stable over time, and no silent drift to nofollow or noindex. A health check surfaces the exceptions so you can act on them." } },
        { "@type": "Question", "name": "How do I improve my backlink health?", "acceptedAnswer": { "@type": "Answer", "text": "Recover lost links while the change is fresh, fix broken destinations with redirects, ask publishers to undo unexpected nofollow or anchor changes, and keep monitoring so new problems reach you early." } },
        { "@type": "Question", "name": "How often should I check my backlink health?", "acceptedAnswer": { "@type": "Answer", "text": "Links can change at any time, so a one-off audit goes stale quickly. Continuous monitoring keeps the picture current and alerts you to changes as they happen." } }
      ]
    }
  ]
}`;

const MAIN = `
<div class="wrap">
  <nav class="crumbs" aria-label="Breadcrumb"><a href="/">Home</a><span>›</span>Backlink Health Check</nav>
</div>

<!-- HERO + DASHBOARD -->
<section aria-labelledby="h1">
  <div class="wrap center">
    <span class="badge">Backlink health check</span>
    <h1 id="h1">Backlink Health Check: See the State of Your Entire Link Profile</h1>
    <p class="lead">It's not just "does this link exist?" Know how healthy your whole backlink portfolio is, and which links need attention first.</p>

    <div class="health" role="group" aria-label="Example backlink health dashboard">
      <p class="micro verify-label">Example data</p>
      <div class="health-top">
        <div class="health-score">
          <small>Overall Health</small>
          <b>93%</b>
          <span>1,284 monitored backlinks</span>
        </div>
        <div class="health-main">
          <div class="health-bar" role="img" aria-label="1,198 healthy, 43 lost, 17 changed, 12 redirected, 8 nofollow, 6 broken">
            <i style="width:93.3%;background:var(--accent)"></i>
            <i style="width:3.35%;background:var(--danger)"></i>
            <i style="width:1.32%;background:var(--warn)"></i>
            <i style="width:.93%;background:#1c7ed6"></i>
            <i style="width:.62%;background:#9c36b5"></i>
            <i style="width:.47%;background:#495057"></i>
          </div>
          <table class="health-table">
            <caption class="sr-only">Backlinks by status</caption>
            <tbody>
              <tr><th scope="row"><span class="dot" style="background:var(--accent)"></span>Healthy</th><td>1,198</td></tr>
              <tr><th scope="row"><span class="dot" style="background:var(--danger)"></span>Lost</th><td>43</td></tr>
              <tr><th scope="row"><span class="dot" style="background:var(--warn)"></span>Changed</th><td>17</td></tr>
              <tr><th scope="row"><span class="dot" style="background:#1c7ed6"></span>Redirected</th><td>12</td></tr>
              <tr><th scope="row"><span class="dot" style="background:#9c36b5"></span>Nofollow</th><td>8</td></tr>
              <tr><th scope="row"><span class="dot" style="background:#495057"></span>Broken</th><td>6</td></tr>
            </tbody>
          </table>
        </div>
      </div>
    </div>
    <p class="micro" style="margin-top:20px">Figures above are illustrative. <a href="/register">Run a health check on your own links</a>.</p>
  </div>
</section>

<!-- WHAT IT MEASURES -->
<section class="alt" aria-labelledby="measures-h">
  <div class="wrap">
    <div class="center">
      <h2 id="measures-h">What a backlink health check measures</h2>
      <p class="lead">A single link is either there or not. A profile is a distribution, and the distribution is what tells you where to spend your time.</p>
    </div>
    <div class="cards">
      <div class="card"><div class="ic">✓</div><h3>Healthy</h3><p>Live, followed, pointing at the right page with the agreed anchor.</p></div>
      <div class="card"><div class="ic">✕</div><h3>Lost</h3><p>Removed from the page. See <a href="/lost-backlinks">lost backlinks</a>.</p></div>
      <div class="card"><div class="ic">404</div><h3>Broken</h3><p>Points to an error, or sits on a page that errors. See <a href="/broken-backlinks">broken backlinks</a>.</p></div>
      <div class="card"><div class="ic">↪</div><h3>Redirected</h3><p>The link now sends visitors somewhere other than your target.</p></div>
      <div class="card"><div class="ic">rel</div><h3>Nofollow / changed</h3><p>Dofollow switched to nofollow, sponsored or UGC, or the anchor changed.</p></div>
      <div class="card"><div class="ic">NI</div><h3>Noindex</h3><p>The linking page is set to noindex and passes little value.</p></div>
    </div>
  </div>
</section>

<!-- HOW TO RUN -->
<section aria-labelledby="run-h">
  <div class="wrap">
    <div class="prose">
      <h2 id="run-h">How to run a backlink health check</h2>
      <p>You can audit a profile by hand, but it goes stale the moment you finish. The reliable version runs continuously:</p>
      <ol>
        <li><b>Gather the links you care about.</b> Paste them or upload a CSV.</li>
        <li><b>Check each linking page.</b> Status, presence of the link, its attributes, anchor and target.</li>
        <li><b>Group the results by status.</b> Healthy, lost, broken, redirected, nofollow, changed.</li>
        <li><b>Act on the exceptions first.</b> The red and amber rows are where the value is leaking.</li>
        <li><b>Keep it running.</b> Re-check on a schedule so the score stays current and changes reach you early.</li>
      </ol>
      <p>LinkPulz turns the health check into something that runs in the background and tracks your score over time, instead of a report you redo every quarter.</p>
    </div>
  </div>
</section>

<!-- HOW TO IMPROVE -->
<section class="alt" aria-labelledby="improve-h">
  <div class="wrap">
    <div class="prose">
      <h2 id="improve-h">How to improve your backlink health</h2>
      <p>A health score only matters if it points you at action. The fixes follow the categories:</p>
      <ul class="check">
        <li><b>Recover lost links.</b> Contact the owner while the change is fresh, see <a href="/lost-backlinks">lost backlinks</a>.</li>
        <li><b>Fix broken destinations.</b> Add a 301 redirect, or ask the site to update the link, see <a href="/broken-backlinks">broken backlinks</a>.</li>
        <li><b>Undo unexpected changes.</b> Ask publishers to restore a dofollow link or the agreed anchor.</li>
        <li><b>Catch new problems early.</b> Monitoring means the next drop reaches you in minutes, not months.</li>
      </ul>
    </div>
  </div>
</section>

<!-- FAQ -->
<section aria-labelledby="faq-h">
  <div class="wrap">
    <div class="center"><h2 id="faq-h">Backlink health check: frequently asked questions</h2></div>
    <div class="faq">
      <details><summary>What is a backlink health check?</summary><p>A backlink health check reviews the current state of every link pointing to your site, which are live, lost, broken, redirected, nofollow or changed, and summarises it so you can see what needs attention first.</p></details>
      <details><summary>How is a backlink health score calculated?</summary><p>A health score summarises the share of your monitored links that are healthy versus those with a problem.</p></details>
      <details><summary>How do I run a backlink health check?</summary><p>Add the links you want to track to LinkPulz. It checks each linking page on a schedule, groups the results by status, and shows what changed, so a health check becomes something that runs continuously rather than once.</p></details>
      <details><summary>What does a healthy backlink profile look like?</summary><p>Most links live, few lost or broken, attributes stable over time, and no silent drift to nofollow or noindex. A health check surfaces the exceptions so you can act on them.</p></details>
      <details><summary>How do I improve my backlink health?</summary><p>Recover lost links while the change is fresh, fix broken destinations with redirects, ask publishers to undo unexpected nofollow or anchor changes, and keep monitoring so new problems reach you early.</p></details>
      <details><summary>How often should I check my backlink health?</summary><p>Links can change at any time, so a one-off audit goes stale quickly. Continuous monitoring keeps the picture current and alerts you to changes as they happen.</p></details>
    </div>
  </div>
</section>

<!-- MORE TOOLS -->
<section class="alt" aria-labelledby="more-h">
  <div class="wrap">
    <div class="center"><h2 id="more-h">More backlink tools and guides</h2></div>
    <div class="cards">
      <div class="card"><h3><a href="/verify-backlinks">Verify a Backlink</a></h3><p>Check whether a single backlink exists, is live and is set up correctly.</p></div>
      <div class="card"><h3><a href="/lost-backlinks">Lost Backlinks</a></h3><p>Find links that have disappeared, understand why, and win them back.</p></div>
      <div class="card"><h3><a href="/broken-backlinks">Broken Backlinks</a></h3><p>Find links that lead to errors, or that sit on pages returning errors.</p></div>
    </div>
  </div>
</section>

<!-- FINAL CTA -->
<section class="final" aria-labelledby="final-h">
  <div class="wrap">
    <h2 id="final-h">Know your backlink health, before it drops.</h2>
    <p class="lead">A profile you don't monitor is a profile you don't know. Track every link with LinkPulz and watch your backlink health in one place.</p>
    <div class="cta-row" style="justify-content:center"><a class="btn" href="/register">Check Your Backlink Health</a></div>
  </div>
</section>
`;

export default function BacklinkHealthCheckPage() {
  return <div dangerouslySetInnerHTML={{ __html: marketingPage(JSONLD, MAIN) }} />;
}
