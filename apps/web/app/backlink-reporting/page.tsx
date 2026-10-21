import type { Metadata } from "next";

export const dynamic = "force-static"; // public content → prerender + edge cache (pairs with the static marketing CSP in middleware.ts)
import { marketingPage } from "../marketing";

// Backlink Reporting, SEO guide/tool page on the shared marketing shell. Unique
// content targeting "backlink reporting" / "backlink reports for clients". Intent:
// communicating backlink status to clients and stakeholders (agencies, in-house,
// freelancers), distinct from management, audit and profile. TO CONFIRM markers
// flag reporting-feature claims (scheduling, white-label, export) to verify.
export const metadata: Metadata = {
  title: { absolute: "Backlink Reporting: Reports for Clients & Teams | LinkPulz" },
  description:
    "Backlink reporting that proves the links are still there: show what's live, what changed and what you recovered. Built for agencies, in-house SEOs and freelancers.",
  alternates: { canonical: "/backlink-reporting", languages: { en: "/backlink-reporting", "x-default": "/backlink-reporting" } },
  robots: { index: true, follow: true, "max-image-preview": "large" },
  openGraph: {
    siteName: "LinkPulz",
    type: "website",
    title: "Backlink Reporting: Reports for Clients & Teams | LinkPulz",
    description:
      "Backlink reporting that proves the links are still there, what's live, what changed, what you recovered. For agencies, in-house SEOs and freelancers.",
    url: "/backlink-reporting",
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
      "@id": "https://www.linkpulz.com/backlink-reporting#webpage",
      "url": "https://www.linkpulz.com/backlink-reporting",
      "name": "Backlink Reporting: Reports for Clients & Teams | LinkPulz",
      "description": "Backlink reporting that proves the links are still there: show what's live, what changed and what you recovered. Built for agencies, in-house SEOs and freelancers.",
      "isPartOf": { "@id": "https://www.linkpulz.com/#website" },
      "about": { "@id": "https://www.linkpulz.com/#software" },
      "breadcrumb": { "@id": "https://www.linkpulz.com/backlink-reporting#breadcrumb" },
      "inLanguage": "en"
    },
    {
      "@type": "BreadcrumbList",
      "@id": "https://www.linkpulz.com/backlink-reporting#breadcrumb",
      "itemListElement": [
        { "@type": "ListItem", "position": 1, "name": "Home", "item": "https://www.linkpulz.com/" },
        { "@type": "ListItem", "position": 2, "name": "Backlink Reporting", "item": "https://www.linkpulz.com/backlink-reporting" }
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
      "@id": "https://www.linkpulz.com/backlink-reporting#faq",
      "mainEntity": [
        { "@type": "Question", "name": "What is backlink reporting?", "acceptedAnswer": { "@type": "Answer", "text": "Backlink reporting is the practice of communicating the state of a site's backlinks to a client or stakeholder: which links are live, which have changed or been lost, and what has been done about them. It turns ongoing monitoring into something you can show." } },
        { "@type": "Question", "name": "What should a backlink report include?", "acceptedAnswer": { "@type": "Answer", "text": "The links being tracked and their current status, what changed since the last report, any links lost or recovered in the period, and the dofollow and live counts. The goal is a clear before-and-after, not a raw data dump." } },
        { "@type": "Question", "name": "Who needs backlink reporting?", "acceptedAnswer": { "@type": "Answer", "text": "SEO agencies reporting to clients, in-house SEOs reporting to stakeholders, and freelancers proving the value of link-building work. Anyone who has to show that the links they earned or paid for are still delivering." } },
        { "@type": "Question", "name": "How often should I send backlink reports?", "acceptedAnswer": { "@type": "Answer", "text": "Monthly is a common cadence for client reporting. The key is that the report reflects fresh data, links re-checked recently, rather than numbers that were true weeks ago when you last looked by hand." } },
        { "@type": "Question", "name": "Can backlink reports be white-labelled?", "acceptedAnswer": { "@type": "Answer", "text": "LinkPulz doesn't generate white-labelled reports itself yet. You can export your links and their full change history as CSV and use it in your own client reports, organised by project or client." } },
        { "@type": "Question", "name": "How does LinkPulz help with backlink reporting?", "acceptedAnswer": { "@type": "Answer", "text": "LinkPulz continuously re-checks each link you add and records what changed, so the data behind a report is always current. You can organise links by project or client and report on each." } }
      ]
    }
  ]
}`;

const MAIN = `
<div class="wrap">
  <nav class="crumbs" aria-label="Breadcrumb"><a href="/">Home</a><span>›</span>Backlink Reporting</nav>
</div>

<!-- HERO -->
<section class="hero" aria-labelledby="h1">
  <div class="wrap grid">
    <div>
      <span class="badge">Backlink reporting</span>
      <h1 id="h1">Backlink Reporting: Show the Links You're Protecting</h1>
      <p class="lead">Earning the link is half the job; proving it's still there is the other half. Backlink reporting turns ongoing monitoring into a clear story for clients and stakeholders, what's live, what changed, and what you recovered.</p>
      <div class="cta-row">
        <a class="btn" href="/register">Report on my backlinks</a>
        <a class="btn ghost" href="#whats-in">What's in a report</a>
      </div>
      <p class="micro">No credit card required</p>
    </div>
    <div class="health" role="img" aria-label="Example backlink report summary for a client, with live, changed and recovered counts" style="margin:0">
      <p class="micro verify-label">Client report, this month</p>
      <div class="health-top">
        <div class="health-score"><small>Links live</small><b>128</b><span>of 134 tracked</span></div>
        <div class="health-main">
          <table class="health-table">
            <tbody>
              <tr><th><span class="dot" style="background:var(--accent)"></span>Live &amp; dofollow</th><td>121</td></tr>
              <tr><th><span class="dot" style="background:var(--warn)"></span>Changed this month</th><td>4</td></tr>
              <tr><th><span class="dot" style="background:var(--danger)"></span>Lost this month</th><td>2</td></tr>
              <tr><th><span class="dot" style="background:var(--brand)"></span>Recovered</th><td>1</td></tr>
            </tbody>
          </table>
        </div>
      </div>
    </div>
  </div>
</section>

<!-- WHAT IT IS -->
<section class="alt" aria-labelledby="what-h">
  <div class="wrap">
    <div class="prose">
      <h2 id="what-h">What is backlink reporting?</h2>
      <p>Backlink reporting is how you communicate the state of a site's backlinks to someone who is paying attention to results, a client, a manager, or yourself at the end of a quarter. It answers: are the links we earned still there, and what has changed?</p>
      <p>It is the visible output of <a href="/backlink-management">backlink management</a>. Monitoring does the watching; the report turns that watching into a story a non-specialist can follow.</p>
    </div>
  </div>
</section>

<!-- WHATS IN A REPORT -->
<section id="whats-in" aria-labelledby="in-h">
  <div class="wrap">
    <div class="prose">
      <h2 id="in-h">What a good backlink report includes</h2>
      <p>A report is persuasive when it shows change clearly, not when it dumps every data point:</p>
      <ul class="check">
        <li><b>The links being tracked</b> and the current status of each.</li>
        <li><b>What changed since last time</b>, the before-and-after is the whole point.</li>
        <li><b>Links lost in the period</b>, with the likely cause. See <a href="/lost-backlinks">lost backlinks</a>.</li>
        <li><b>Links recovered or fixed</b>, the work you actually did.</li>
        <li><b>Live and dofollow counts</b>, so value is easy to read at a glance.</li>
        <li><b>A short narrative.</b> Two lines of context beat a spreadsheet nobody opens.</li>
      </ul>
    </div>
  </div>
</section>

<!-- WHO ITS FOR -->
<section class="alt" aria-labelledby="who-h">
  <div class="wrap">
    <div class="center"><h2 id="who-h">Who backlink reporting is for</h2></div>
    <div class="cards">
      <div class="card"><div class="ic">A</div><h3>Agencies</h3><p>Prove to clients each month that the links you built are still live, and flag the ones you saved.</p></div>
      <div class="card"><div class="ic">H</div><h3>In-house SEOs</h3><p>Show stakeholders that your link-building budget is holding its value, not quietly leaking away.</p></div>
      <div class="card"><div class="ic">F</div><h3>Freelancers</h3><p>Turn invisible maintenance work into a report that justifies the retainer.</p></div>
    </div>
  </div>
</section>

<!-- MANUAL VS AUTOMATED -->
<section aria-labelledby="how-h">
  <div class="wrap">
    <div class="prose">
      <h2 id="how-h">Reporting by hand vs from monitoring</h2>
      <h3>By hand</h3>
      <p>Re-check each link before every report, copy the results into a document, and hope nothing changed between finishing the report and the client reading it. It is slow, and the data is stale on arrival.</p>
      <h3>From monitoring</h3>
      <p>When your links are already monitored, the report writes most of itself: the data is current because it is re-checked continuously, and the change log is already there. Organise links by project or client and report on each.</p>
    </div>
  </div>
</section>

<!-- FAQ -->
<section class="alt" aria-labelledby="faq-h">
  <div class="wrap">
    <div class="center"><h2 id="faq-h">Backlink reporting: frequently asked questions</h2></div>
    <div class="faq">
      <details><summary>What is backlink reporting?</summary><p>Backlink reporting is the practice of communicating the state of a site's backlinks to a client or stakeholder: which links are live, which have changed or been lost, and what has been done about them. It turns ongoing monitoring into something you can show.</p></details>
      <details><summary>What should a backlink report include?</summary><p>The links being tracked and their current status, what changed since the last report, any links lost or recovered in the period, and the dofollow and live counts. The goal is a clear before-and-after, not a raw data dump.</p></details>
      <details><summary>Who needs backlink reporting?</summary><p>SEO agencies reporting to clients, in-house SEOs reporting to stakeholders, and freelancers proving the value of link-building work. Anyone who has to show that the links they earned or paid for are still delivering.</p></details>
      <details><summary>How often should I send backlink reports?</summary><p>Monthly is a common cadence for client reporting. The key is that the report reflects fresh data, links re-checked recently, rather than numbers that were true weeks ago when you last looked by hand.</p></details>
      <details><summary>Can backlink reports be white-labelled?</summary><p>LinkPulz doesn't generate white-labelled reports itself yet. You can export your links and their full change history as CSV and use it in your own client reports, organised by project or client.</p></details>
      <details><summary>How does LinkPulz help with backlink reporting?</summary><p>LinkPulz continuously re-checks each link you add and records what changed, so the data behind a report is always current. You can organise links by project or client and report on each.</p></details>
    </div>
  </div>
</section>

<!-- MORE TOOLS -->
<section aria-labelledby="more-h">
  <div class="wrap">
    <div class="center"><h2 id="more-h">More backlink tools and guides</h2></div>
    <div class="cards">
      <div class="card"><h3><a href="/backlink-management">Backlink Management</a></h3><p>Keep every link in one place and monitor it over time.</p></div>
      <div class="card"><h3><a href="/backlink-health-check">Backlink Health Check</a></h3><p>See the overall state of your backlink portfolio at a glance.</p></div>
      <div class="card"><h3><a href="/backlink-profile">Backlink Profile</a></h3><p>Understand the full picture: dofollow ratio, anchors and referring domains.</p></div>
    </div>
  </div>
</section>

<!-- FINAL CTA -->
<section class="final" aria-labelledby="final-h">
  <div class="wrap">
    <h2 id="final-h">Report on links that are actually still there.</h2>
    <p class="lead">Monitor your backlinks with LinkPulz and report from data that's current, so every client report proves the links are live, not that they were live last month.</p>
    <div class="cta-row" style="justify-content:center"><a class="btn" href="/register">Report on my backlinks</a></div>
  </div>
</section>
`;

export default function BacklinkReportingPage() {
  return <div dangerouslySetInnerHTML={{ __html: marketingPage(JSONLD, MAIN) }} />;
}
