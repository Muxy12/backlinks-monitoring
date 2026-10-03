import type { Metadata } from "next";
import { marketingPage } from "../marketing";

// Lost Backlinks — SEO tool/guide page. Built on the shared marketing shell so
// the charte, header and footer match the homepage. Unique content targeting
// "lost backlinks" / "lost backlink checker". TO CONFIRM markers flag claims to
// verify before launch (check frequency, alert channels, recovery workflow).
export const metadata: Metadata = {
  title: { absolute: "Lost Backlinks: Find & Recover Lost Links | LinkPulz" },
  description:
    "A lost backlink checker that tells you which links disappeared, when and why. Find lost backlinks, recover them, and get alerted the next time one drops.",
  alternates: { canonical: "/lost-backlinks" },
  robots: { index: true, follow: true, "max-image-preview": "large" },
  openGraph: {
    type: "website",
    title: "Lost Backlinks: Find & Recover Lost Links | LinkPulz",
    description:
      "Find lost backlinks, understand why they disappeared, recover them, and get alerted the next time a link drops.",
    url: "/lost-backlinks",
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
      "@id": "https://www.linkpulz.com/lost-backlinks#webpage",
      "url": "https://www.linkpulz.com/lost-backlinks",
      "name": "Lost Backlinks: Find & Recover Lost Links | LinkPulz",
      "description": "A lost backlink checker that tells you which links disappeared, when and why. Find lost backlinks, recover them, and get alerted the next time one drops.",
      "isPartOf": { "@id": "https://www.linkpulz.com/#website" },
      "about": { "@id": "https://www.linkpulz.com/#software" },
      "breadcrumb": { "@id": "https://www.linkpulz.com/lost-backlinks#breadcrumb" },
      "inLanguage": "en"
    },
    {
      "@type": "BreadcrumbList",
      "@id": "https://www.linkpulz.com/lost-backlinks#breadcrumb",
      "itemListElement": [
        { "@type": "ListItem", "position": 1, "name": "Home", "item": "https://www.linkpulz.com/" },
        { "@type": "ListItem", "position": 2, "name": "Lost Backlinks", "item": "https://www.linkpulz.com/lost-backlinks" }
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
      "@id": "https://www.linkpulz.com/lost-backlinks#faq",
      "mainEntity": [
        { "@type": "Question", "name": "What is a lost backlink?", "acceptedAnswer": { "@type": "Answer", "text": "A lost backlink is a link that used to point to your site and is no longer there. The linking page may have removed the link, been deleted, been rewritten, or the link may have been replaced or redirected elsewhere." } },
        { "@type": "Question", "name": "How do I find lost backlinks?", "acceptedAnswer": { "@type": "Answer", "text": "Compare a past list of your backlinks with the live state of each linking page. A lost backlink checker re-fetches each page and flags any link that is no longer present, so you do not have to check them by hand." } },
        { "@type": "Question", "name": "Why do backlinks disappear?", "acceptedAnswer": { "@type": "Answer", "text": "Common causes are the publisher removing the link, the page being deleted or returning a 404, the article being edited or rewritten, a site migration breaking the URL, a redirect sending the link elsewhere, or the link switching to nofollow or the page going noindex." } },
        { "@type": "Question", "name": "Do lost backlinks hurt my rankings?", "acceptedAnswer": { "@type": "Answer", "text": "Losing a strong link can reduce the authority flowing to your pages and the referral traffic it sent. The impact depends on the link's value and how many you lose, so catching losses early matters." } },
        { "@type": "Question", "name": "Can I recover a lost backlink?", "acceptedAnswer": { "@type": "Answer", "text": "Often, yes. If you know who owns the page, when the link disappeared and why, you can contact the site and ask them to restore it while the change is still recent. [TO CONFIRM: recovery workflow]" } },
        { "@type": "Question", "name": "How does LinkPulz detect lost backlinks?", "acceptedAnswer": { "@type": "Answer", "text": "Add your links to LinkPulz. It re-checks each linking page on a schedule and alerts you the moment a link is no longer there, with the likely cause. [TO CONFIRM: check frequency and alert channels]" } }
      ]
    }
  ]
}`;

const MAIN = `
<div class="wrap">
  <nav class="crumbs" aria-label="Breadcrumb"><a href="/">Home</a><span>›</span>Lost Backlinks</nav>
</div>

<!-- HERO -->
<section class="hero" aria-labelledby="h1">
  <div class="wrap grid">
    <div>
      <span class="badge">Lost backlink checker</span>
      <h1 id="h1">Lost Backlinks: Find the Links That Disappeared</h1>
      <p class="lead">A lost backlink is a link you earned or paid for that is no longer on the page. Find out which links disappeared, when, and why, so you can win them back before they cost you rankings and referral traffic.</p>
      <div class="cta-row">
        <a class="btn" href="/register">Get Alerted When a Backlink Disappears</a>
        <a class="btn ghost" href="#how-to-find">How to find them</a>
      </div>
      <p class="micro">No credit card required <span class="tc">TO CONFIRM</span></p>
    </div>
    <div class="lost-demo" role="img" aria-label="Example: a backlink goes from live yesterday to lost today, with a LinkPulz alert">
      <p class="micro verify-label">Example</p>
      <div class="lost-row"><small>Yesterday</small><span class="url">example.com/article</span><span class="status live">● LIVE</span></div>
      <div class="lost-row"><small>Today</small><span class="url">example.com/article</span><span class="status lost">● LOST</span></div>
      <div class="lost-alert"><b>LinkPulz Alert</b><br>Backlink lost 23 minutes ago.</div>
    </div>
  </div>
</section>

<!-- WHY DISAPPEAR -->
<section class="alt" aria-labelledby="why-h">
  <div class="wrap">
    <div class="prose">
      <h2 id="why-h">Why backlinks disappear</h2>
      <p>Links rarely announce themselves when they leave. A page you were linked from can change in half a dozen ways, and most of them are silent:</p>
      <ul class="check">
        <li><b>The publisher removes the link.</b> An editor cleans up outbound links, or the relationship ends.</li>
        <li><b>The page is deleted or returns a 404.</b> The whole article is gone, taking your link with it.</li>
        <li><b>The article is edited or rewritten.</b> A refresh drops the paragraph that held your link.</li>
        <li><b>A site migration breaks the URL.</b> New CMS, new URL structure, no redirect.</li>
        <li><b>A bad redirect sends the link elsewhere.</b> The link still exists but no longer points at you.</li>
        <li><b>It switches to nofollow, or the page goes noindex.</b> The link is there but stops passing value.</li>
      </ul>
    </div>
  </div>
</section>

<!-- HOW TO FIND -->
<section id="how-to-find" aria-labelledby="find-h">
  <div class="wrap">
    <div class="prose">
      <h2 id="find-h">How to find lost backlinks</h2>
      <p>Finding one lost link is easy; finding them across a whole portfolio is where it breaks down.</p>
      <h3>Manually</h3>
      <ol>
        <li><b>Start from a known list.</b> Export your backlinks from a past report, a spreadsheet or an SEO tool.</li>
        <li><b>Open each linking page.</b> Look for your link, or view the page source and search for your domain.</li>
        <li><b>Check the status.</b> Note pages that 404, redirect, or no longer contain the link.</li>
        <li><b>Record what changed and when.</b> A date makes the recovery conversation far easier.</li>
      </ol>
      <p>This works for a handful of links. For dozens or hundreds, it does not scale, and you only ever see the state of a link on the day you happen to check.</p>
      <h3>With monitoring</h3>
      <p>Add your links to LinkPulz once. It re-checks every linking page on a schedule and flags any link that is no longer there, with the likely cause, so a loss reaches you in minutes instead of months. <span class="tc">TO CONFIRM: check frequency and alert channels</span></p>
    </div>
  </div>
</section>

<!-- HOW TO RECOVER -->
<section class="alt" aria-labelledby="recover-h">
  <div class="wrap">
    <div class="prose">
      <h2 id="recover-h">How to recover a lost backlink</h2>
      <p>A lost link is often recoverable, and the odds are far better while the change is fresh:</p>
      <ol>
        <li><b>Confirm it is really gone.</b> Rule out a temporary error or a page that was just slow to load.</li>
        <li><b>Find out why.</b> Removed on purpose, lost in an edit, or broken by a migration each need a different ask.</li>
        <li><b>Identify who owns the page.</b> The author, the editor or the webmaster, depending on the site.</li>
        <li><b>Reach out with the specifics.</b> Link, date, and what changed — and a reason it is worth restoring.</li>
        <li><b>Track the outcome.</b> Keep each attempt until the link is back. <span class="tc">TO CONFIRM: recovery workflow feature</span></li>
      </ol>
    </div>
  </div>
</section>

<!-- LOST VS OTHER -->
<section aria-labelledby="vs-h">
  <div class="wrap">
    <div class="prose">
      <h2 id="vs-h">Lost, broken or changed?</h2>
      <p>Not every problem link is a lost one, and the fix is different for each:</p>
      <ul class="check">
        <li><b>Lost</b> — the link is gone from the page. Covered here.</li>
        <li><b>Broken</b> — the link still exists but points to an error, or sits on a page that errors. See <a href="/broken-backlinks">broken backlinks</a>.</li>
        <li><b>Changed</b> — the link is there but the anchor, target or rel attribute changed. See the <a href="/backlink-health-check">backlink health check</a>.</li>
      </ul>
      <p>You can also <a href="/verify-backlinks">verify a single backlink</a> on demand.</p>
    </div>
  </div>
</section>

<!-- FAQ -->
<section class="alt" aria-labelledby="faq-h">
  <div class="wrap">
    <div class="center"><h2 id="faq-h">Lost backlinks: frequently asked questions</h2></div>
    <div class="faq">
      <details><summary>What is a lost backlink?</summary><p>A lost backlink is a link that used to point to your site and is no longer there. The linking page may have removed the link, been deleted, been rewritten, or the link may have been replaced or redirected elsewhere.</p></details>
      <details><summary>How do I find lost backlinks?</summary><p>Compare a past list of your backlinks with the live state of each linking page. A lost backlink checker re-fetches each page and flags any link that is no longer present, so you do not have to check them by hand.</p></details>
      <details><summary>Why do backlinks disappear?</summary><p>Common causes are the publisher removing the link, the page being deleted or returning a 404, the article being edited or rewritten, a site migration breaking the URL, a redirect sending the link elsewhere, or the link switching to nofollow or the page going noindex.</p></details>
      <details><summary>Do lost backlinks hurt my rankings?</summary><p>Losing a strong link can reduce the authority flowing to your pages and the referral traffic it sent. The impact depends on the link's value and how many you lose, so catching losses early matters.</p></details>
      <details><summary>Can I recover a lost backlink?</summary><p>Often, yes. If you know who owns the page, when the link disappeared and why, you can contact the site and ask them to restore it while the change is still recent. <span class="tc">TO CONFIRM: recovery workflow</span></p></details>
      <details><summary>How does LinkPulz detect lost backlinks?</summary><p>Add your links to LinkPulz. It re-checks each linking page on a schedule and alerts you the moment a link is no longer there, with the likely cause. <span class="tc">TO CONFIRM: check frequency and alert channels</span></p></details>
    </div>
  </div>
</section>

<!-- MORE TOOLS -->
<section aria-labelledby="more-h">
  <div class="wrap">
    <div class="center"><h2 id="more-h">More backlink tools and guides</h2></div>
    <div class="cards">
      <div class="card"><h3><a href="/verify-backlinks">Verify a Backlink</a></h3><p>Check whether a single backlink exists, is live and is set up correctly.</p></div>
      <div class="card"><h3><a href="/broken-backlinks">Broken Backlinks</a></h3><p>Find links that lead to errors, or that sit on pages returning errors.</p></div>
      <div class="card"><h3><a href="/backlink-health-check">Backlink Health Check</a></h3><p>See the overall state of your backlink portfolio at a glance.</p></div>
    </div>
  </div>
</section>

<!-- FINAL CTA -->
<section class="final" aria-labelledby="final-h">
  <div class="wrap">
    <h2 id="final-h">Stop losing links in silence.</h2>
    <p class="lead">Every lost backlink is lost authority and lost traffic you already paid for. Monitor your links with LinkPulz and get alerted the moment one disappears.</p>
    <div class="cta-row" style="justify-content:center"><a class="btn" href="/register">Monitor my backlinks</a></div>
  </div>
</section>
`;

export default function LostBacklinksPage() {
  return <div dangerouslySetInnerHTML={{ __html: marketingPage(JSONLD, MAIN) }} />;
}
