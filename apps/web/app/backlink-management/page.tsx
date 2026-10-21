import type { Metadata } from "next";

export const dynamic = "force-static"; // public content → prerender + edge cache (pairs with the static marketing CSP in middleware.ts)
import { marketingPage } from "../marketing";

// Backlink Management, SEO tool/guide page on the shared marketing shell.
// Unique content targeting "backlink management" / "backlink management tool".
// Intent: ongoing operations, organise and monitor a whole backlink portfolio
// (distinct from the one-off audit and the point-in-time profile pages).
// TO CONFIRM markers flag feature claims to verify before launch.
export const metadata: Metadata = {
  title: { absolute: "Backlink Management: Track & Monitor Your Links | LinkPulz" },
  description:
    "Backlink management done right: keep every earned and paid link in one place, monitor it on a schedule, and get alerted the moment a link is lost, nofollowed or broken.",
  alternates: { canonical: "/backlink-management", languages: { en: "/backlink-management", "x-default": "/backlink-management" } },
  robots: { index: true, follow: true, "max-image-preview": "large" },
  openGraph: {
    siteName: "LinkPulz",
    type: "website",
    title: "Backlink Management: Track & Monitor Your Links | LinkPulz",
    description:
      "Keep every backlink you've earned in one place, monitor it on a schedule, and get alerted the moment a link changes.",
    url: "/backlink-management",
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
      "@id": "https://www.linkpulz.com/backlink-management#webpage",
      "url": "https://www.linkpulz.com/backlink-management",
      "name": "Backlink Management: Track & Monitor Your Links | LinkPulz",
      "description": "Backlink management done right: keep every earned and paid link in one place, monitor it on a schedule, and get alerted the moment a link is lost, nofollowed or broken.",
      "isPartOf": { "@id": "https://www.linkpulz.com/#website" },
      "about": { "@id": "https://www.linkpulz.com/#software" },
      "breadcrumb": { "@id": "https://www.linkpulz.com/backlink-management#breadcrumb" },
      "inLanguage": "en"
    },
    {
      "@type": "BreadcrumbList",
      "@id": "https://www.linkpulz.com/backlink-management#breadcrumb",
      "itemListElement": [
        { "@type": "ListItem", "position": 1, "name": "Home", "item": "https://www.linkpulz.com/" },
        { "@type": "ListItem", "position": 2, "name": "Backlink Management", "item": "https://www.linkpulz.com/backlink-management" }
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
      "@id": "https://www.linkpulz.com/backlink-management#faq",
      "mainEntity": [
        { "@type": "Question", "name": "What is backlink management?", "acceptedAnswer": { "@type": "Answer", "text": "Backlink management is the ongoing practice of keeping track of the links pointing to your site: knowing which ones you have, whether they are still live and set up correctly, and acting when one changes. It is maintenance, not a one-off report." } },
        { "@type": "Question", "name": "Why does backlink management matter?", "acceptedAnswer": { "@type": "Answer", "text": "Backlinks are assets you earned or paid for, and they decay silently. A link can be removed, nofollowed or broken without any notice. Managing them means you keep the authority and referral traffic you already worked for instead of losing it unnoticed." } },
        { "@type": "Question", "name": "What should I track for each backlink?", "acceptedAnswer": { "@type": "Answer", "text": "At minimum: the linking page URL, your target URL, the anchor text, whether the link is live, and whether it is dofollow or nofollow. Tracking the first-seen date and the last-checked date makes it clear when something changed." } },
        { "@type": "Question", "name": "Can I manage backlinks in a spreadsheet?", "acceptedAnswer": { "@type": "Answer", "text": "You can list them in a spreadsheet, but a spreadsheet cannot tell you when a link changes. It only ever shows the state on the day you last checked by hand. Monitoring software re-checks each link for you and flags changes automatically." } },
        { "@type": "Question", "name": "How is backlink management different from a backlink audit?", "acceptedAnswer": { "@type": "Answer", "text": "An audit is a point-in-time review that finds problems in your profile right now. Management is the continuous process that keeps the profile healthy between audits, so new problems are caught as they happen rather than at the next review." } },
        { "@type": "Question", "name": "How does LinkPulz help manage backlinks?", "acceptedAnswer": { "@type": "Answer", "text": "Add your links to LinkPulz once. It keeps them in one place, re-checks each linking page on a schedule, and alerts you when a link is lost, redirected, nofollowed, noindexed or broken." } }
      ]
    }
  ]
}`;

const MAIN = `
<div class="wrap">
  <nav class="crumbs" aria-label="Breadcrumb"><a href="/">Home</a><span>›</span>Backlink Management</nav>
</div>

<!-- HERO -->
<section class="hero" aria-labelledby="h1">
  <div class="wrap grid">
    <div>
      <span class="badge">Backlink management</span>
      <h1 id="h1">Backlink Management: Keep Every Link You've Earned Working</h1>
      <p class="lead">Every backlink you earned or paid for is an asset, and assets decay in silence. Backlink management is keeping them all in one place, watching them on a schedule, and acting the moment one changes, so you never lose authority you already paid for.</p>
      <div class="cta-row">
        <a class="btn" href="/register">Start managing my backlinks</a>
        <a class="btn ghost" href="#what-to-track">What to track</a>
      </div>
      <p class="micro">No credit card required</p>
    </div>
    <div class="lost-demo" role="img" aria-label="Example backlink portfolio: three links with live, nofollow and lost statuses">
      <p class="micro verify-label">Your portfolio</p>
      <div class="lost-row"><small>Live</small><span class="url">forbes.com/article</span><span class="status live">● DOFOLLOW</span></div>
      <div class="lost-row"><small>Changed</small><span class="url">techblog.com/review</span><span class="status lost">● NOFOLLOW</span></div>
      <div class="lost-row"><small>Gone</small><span class="url">news.example.com/post</span><span class="status lost">● LOST</span></div>
      <div class="lost-alert"><b>LinkPulz Alert</b><br>2 links changed since your last check.</div>
    </div>
  </div>
</section>

<!-- WHAT IT IS -->
<section class="alt" aria-labelledby="what-h">
  <div class="wrap">
    <div class="prose">
      <h2 id="what-h">What is backlink management?</h2>
      <p>Backlink management is the ongoing practice of keeping track of the links that point to your site, knowing which ones you have, whether each is still live and set up the way you expect, and doing something about it when one changes.</p>
      <p>It is easy to confuse with a one-off <a href="/backlink-audit">backlink audit</a>, but they are different jobs. An audit is a snapshot that finds problems today. Management is the maintenance that keeps the <a href="/backlink-profile">backlink profile</a> healthy between audits, so problems surface the day they happen instead of at your next review.</p>
    </div>
  </div>
</section>

<!-- WHY IT MATTERS -->
<section aria-labelledby="why-h">
  <div class="wrap">
    <div class="prose">
      <h2 id="why-h">Why it matters</h2>
      <p>A backlink is not a one-time win. Once it is live, anything can happen to it, and none of it reaches your inbox automatically:</p>
      <ul class="check">
        <li><b>Links disappear.</b> An editor removes them, the page is deleted, or an article is rewritten. See <a href="/lost-backlinks">lost backlinks</a>.</li>
        <li><b>Links break.</b> Your target starts returning a 404, or the linking page itself errors. See <a href="/broken-backlinks">broken backlinks</a>.</li>
        <li><b>Links quietly change.</b> Dofollow becomes nofollow, the anchor is edited, or the page goes noindex, the link is there but stops passing value.</li>
        <li><b>You lose the context.</b> Six months later, nobody remembers which links were paid for, which were earned, or who to contact when one drops.</li>
      </ul>
      <p>Unmanaged, a backlink profile leaks value one link at a time. Managed, every change is caught while it is still fresh and still recoverable.</p>
    </div>
  </div>
</section>

<!-- WHAT TO TRACK -->
<section class="alt" id="what-to-track" aria-labelledby="track-h">
  <div class="wrap">
    <div class="prose">
      <h2 id="track-h">What to track for every backlink</h2>
      <p>Good management starts with recording the right details for each link, so a change is obvious the moment it happens:</p>
    </div>
    <div class="tablewrap">
      <table>
        <thead><tr><th>Field</th><th>Why it matters</th></tr></thead>
        <tbody>
          <tr><th>Linking page URL</th><td>The page the link lives on, the thing you re-check.</td></tr>
          <tr><th>Your target URL</th><td>Where the link points. Reveals redirects and wrong targets.</td></tr>
          <tr><th>Anchor text</th><td>Changes to the anchor change the signal the link sends.</td></tr>
          <tr><th>Dofollow / nofollow</th><td>Tells you whether the link still passes authority.</td></tr>
          <tr><th>Status</th><td>Live, lost, redirected or broken, the heart of the matter.</td></tr>
          <tr><th>First seen &amp; last checked</th><td>Dates make it clear exactly when something changed.</td></tr>
        </tbody>
      </table>
    </div>
  </div>
</section>

<!-- MANUAL VS TOOL -->
<section aria-labelledby="how-h">
  <div class="wrap">
    <div class="prose">
      <h2 id="how-h">Managing by hand vs with monitoring</h2>
      <h3>By hand</h3>
      <ol>
        <li><b>List every link</b> you know about in a spreadsheet.</li>
        <li><b>Open each linking page</b> on a regular cadence and look for your link.</li>
        <li><b>Note what changed</b>, gone, nofollowed, redirected, or broken.</li>
        <li><b>Repeat forever.</b> A spreadsheet only shows the state on the day you last checked, and the checking never ends.</li>
      </ol>
      <h3>With monitoring</h3>
      <p>Add your links to LinkPulz once. It keeps them in one place and re-checks each linking page on a schedule, so a change reaches you in minutes instead of surfacing months later in a report.</p>
    </div>
  </div>
</section>

<!-- FAQ -->
<section class="alt" aria-labelledby="faq-h">
  <div class="wrap">
    <div class="center"><h2 id="faq-h">Backlink management: frequently asked questions</h2></div>
    <div class="faq">
      <details><summary>What is backlink management?</summary><p>Backlink management is the ongoing practice of keeping track of the links pointing to your site: knowing which ones you have, whether they are still live and set up correctly, and acting when one changes. It is maintenance, not a one-off report.</p></details>
      <details><summary>Why does backlink management matter?</summary><p>Backlinks are assets you earned or paid for, and they decay silently. A link can be removed, nofollowed or broken without any notice. Managing them means you keep the authority and referral traffic you already worked for instead of losing it unnoticed.</p></details>
      <details><summary>What should I track for each backlink?</summary><p>At minimum: the linking page URL, your target URL, the anchor text, whether the link is live, and whether it is dofollow or nofollow. Tracking the first-seen date and the last-checked date makes it clear when something changed.</p></details>
      <details><summary>Can I manage backlinks in a spreadsheet?</summary><p>You can list them in a spreadsheet, but a spreadsheet cannot tell you when a link changes. It only ever shows the state on the day you last checked by hand. Monitoring software re-checks each link for you and flags changes automatically.</p></details>
      <details><summary>How is backlink management different from a backlink audit?</summary><p>An audit is a point-in-time review that finds problems in your profile right now. Management is the continuous process that keeps the profile healthy between audits, so new problems are caught as they happen rather than at the next review.</p></details>
      <details><summary>How does LinkPulz help manage backlinks?</summary><p>Add your links to LinkPulz once. It keeps them in one place, re-checks each linking page on a schedule, and alerts you when a link is lost, redirected, nofollowed, noindexed or broken.</p></details>
    </div>
  </div>
</section>

<!-- MORE TOOLS -->
<section aria-labelledby="more-h">
  <div class="wrap">
    <div class="center"><h2 id="more-h">More backlink tools and guides</h2></div>
    <div class="cards">
      <div class="card"><h3><a href="/backlink-audit">Backlink Audit</a></h3><p>Run a point-in-time review to find the problems in your link profile.</p></div>
      <div class="card"><h3><a href="/backlink-profile">Backlink Profile</a></h3><p>Understand the full picture: dofollow ratio, anchors and referring domains.</p></div>
      <div class="card"><h3><a href="/backlink-health-check">Backlink Health Check</a></h3><p>See the overall state of your backlink portfolio at a glance.</p></div>
    </div>
  </div>
</section>

<!-- FINAL CTA -->
<section class="final" aria-labelledby="final-h">
  <div class="wrap">
    <h2 id="final-h">Stop managing backlinks from memory.</h2>
    <p class="lead">Keep every link you've earned in one place and let LinkPulz watch them for you, so the day a link changes, you're the first to know.</p>
    <div class="cta-row" style="justify-content:center"><a class="btn" href="/register">Start managing my backlinks</a></div>
  </div>
</section>
`;

export default function BacklinkManagementPage() {
  return <div dangerouslySetInnerHTML={{ __html: marketingPage(JSONLD, MAIN) }} />;
}
