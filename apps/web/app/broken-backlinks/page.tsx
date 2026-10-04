import type { Metadata } from "next";
import { marketingPage } from "../marketing";

// Broken Backlinks — SEO tool/guide page on the shared marketing shell. Unique
// content targeting "broken backlinks" / "broken backlink checker". TO CONFIRM
// markers flag claims to verify before launch.
export const metadata: Metadata = {
  title: { absolute: "Broken Backlinks: Find & Fix Broken Links | LinkPulz" },
  description:
    "A broken backlink checker that catches links pointing to errors and links on pages that error. Find broken backlinks, fix them, and monitor them automatically.",
  alternates: { canonical: "/broken-backlinks" },
  robots: { index: true, follow: true, "max-image-preview": "large" },
  openGraph: {
    type: "website",
    title: "Broken Backlinks: Find & Fix Broken Links | LinkPulz",
    description:
      "Find broken backlinks, links pointing to errors and links on pages that error, fix them, and monitor them automatically.",
    url: "/broken-backlinks",
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
      "@id": "https://www.linkpulz.com/broken-backlinks#webpage",
      "url": "https://www.linkpulz.com/broken-backlinks",
      "name": "Broken Backlinks: Find & Fix Broken Links | LinkPulz",
      "description": "A broken backlink checker that catches links pointing to errors and links on pages that error. Find broken backlinks, fix them, and monitor them automatically.",
      "isPartOf": { "@id": "https://www.linkpulz.com/#website" },
      "about": { "@id": "https://www.linkpulz.com/#software" },
      "breadcrumb": { "@id": "https://www.linkpulz.com/broken-backlinks#breadcrumb" },
      "inLanguage": "en"
    },
    {
      "@type": "BreadcrumbList",
      "@id": "https://www.linkpulz.com/broken-backlinks#breadcrumb",
      "itemListElement": [
        { "@type": "ListItem", "position": 1, "name": "Home", "item": "https://www.linkpulz.com/" },
        { "@type": "ListItem", "position": 2, "name": "Broken Backlinks", "item": "https://www.linkpulz.com/broken-backlinks" }
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
      "@id": "https://www.linkpulz.com/broken-backlinks#faq",
      "mainEntity": [
        { "@type": "Question", "name": "What is a broken backlink?", "acceptedAnswer": { "@type": "Answer", "text": "A broken backlink is a link that no longer works as intended. Either the link points to a URL that returns an error (such as a 404), or the link sits on a page that itself returns an error, so the link is unreachable." } },
        { "@type": "Question", "name": "How are broken backlinks different from lost backlinks?", "acceptedAnswer": { "@type": "Answer", "text": "A lost backlink has been removed from the page. A broken backlink is still there, but it points to an error or lives on a page that errors, so it no longer passes value or sends traffic." } },
        { "@type": "Question", "name": "How do I find broken backlinks?", "acceptedAnswer": { "@type": "Answer", "text": "Check two things for each link: the HTTP status of the page that links to you, and the HTTP status of the URL it points to. A broken backlink checker fetches both and flags any that return an error or a redirect." } },
        { "@type": "Question", "name": "How do I fix a broken backlink?", "acceptedAnswer": { "@type": "Answer", "text": "If the broken URL is on your own site, add a 301 redirect from the old URL to the right page so the link keeps its value. If the linking page itself is broken, contact the site owner to restore the page or update the link." } },
        { "@type": "Question", "name": "Do broken backlinks hurt SEO?", "acceptedAnswer": { "@type": "Answer", "text": "A link that points to a 404 on your site wastes the authority it could have passed, and a dead linking page sends no traffic. Fixing broken links recovers value you already earned." } },
        { "@type": "Question", "name": "Can LinkPulz find broken backlinks automatically?", "acceptedAnswer": { "@type": "Answer", "text": "Yes. LinkPulz monitors the status of your backlinks and their destinations and alerts you when either side starts returning an error. [TO CONFIRM: that both sides are checked, and alert channels]" } }
      ]
    }
  ]
}`;

const MAIN = `
<div class="wrap">
  <nav class="crumbs" aria-label="Breadcrumb"><a href="/">Home</a><span>›</span>Broken Backlinks</nav>
</div>

<!-- HERO -->
<section aria-labelledby="h1">
  <div class="wrap center">
    <span class="badge">Broken backlink checker</span>
    <h1 id="h1">Broken Backlinks: Find the Links That Lead to Errors</h1>
    <p class="lead">A broken backlink isn't always a lost one. The link can still be there while the page it points to, or the page it sits on, is returning an error. A broken backlink checker catches both.</p>
    <div class="cta-row" style="justify-content:center">
      <a class="btn" href="/register">Find Broken Backlinks Automatically</a>
      <a class="btn ghost" href="#how-to-fix">How to fix them</a>
    </div>
  </div>
</section>

<!-- TWO CASES -->
<section class="alt" aria-labelledby="cases-h">
  <div class="wrap">
    <div class="center">
      <h2 id="cases-h">Two kinds of broken backlink</h2>
      <p class="lead">Both waste a link you earned, and both are easy to miss without checking the status of each side.</p>
    </div>
    <div class="cards">
      <div class="card broken-case">
        <h3>Case 1: Your link works, but the destination is broken</h3>
        <p>Another site still links to you, but the URL it points to no longer exists.</p>
        <div class="flow" role="img" aria-label="Example: a backlink points to example.com/my-old-page, which returns a 404 error">
          <span class="url">example.com/my-old-page</span>
          <span class="arrow" aria-hidden="true">↓</span>
          <span class="status lost">● 404</span>
        </div>
        <p class="micro">Typical causes: a deleted page, a changed URL, a missing redirect after a migration.</p>
      </div>
      <div class="card broken-case">
        <h3>Case 2: The page holding your link is broken</h3>
        <p>Your backlink is on a page that now returns an error, so the link is no longer reachable.</p>
        <div class="flow" role="img" aria-label="Example: the page that contains your backlink returns a 404 or 5xx error">
          <span class="url">othersite.com/article-with-your-link</span>
          <span class="arrow" aria-hidden="true">↓</span>
          <span class="status lost">● 404 / 5xx</span>
        </div>
        <p class="micro">Typical causes: the publisher deleted the article, a server error, or the site went offline.</p>
      </div>
    </div>
  </div>
</section>

<!-- HOW TO FIND -->
<section aria-labelledby="find-h">
  <div class="wrap">
    <div class="prose">
      <h2 id="find-h">How to find broken backlinks</h2>
      <p>For each backlink, there are two statuses that matter, not one:</p>
      <ul class="check">
        <li>The HTTP status of the <b>page that links to you</b>, does it load, redirect, or error?</li>
        <li>The HTTP status of the <b>URL it points to</b>, does your target page still exist?</li>
        <li>Any <b>redirect</b> on either side that sends visitors somewhere unexpected.</li>
      </ul>
      <p>Checking both by hand is tedious; checking both across a whole portfolio is where a <a href="/verify-backlinks">verification tool</a> or ongoing monitoring earns its keep.</p>
    </div>
  </div>
</section>

<!-- HOW TO FIX -->
<section id="how-to-fix" class="alt" aria-labelledby="fix-h">
  <div class="wrap">
    <div class="prose">
      <h2 id="fix-h">How to fix broken backlinks</h2>
      <p>The fix depends on which side is broken:</p>
      <ol>
        <li><b>Broken destination on your site.</b> Add a <b>301 redirect</b> from the old URL to the correct, live page. The link keeps working and keeps its value.</li>
        <li><b>The target page was removed on purpose.</b> Point the link to the closest relevant page instead of leaving it on a 404.</li>
        <li><b>The linking page is broken.</b> Contact the site owner and ask them to restore the page, or to move the link to a working page.</li>
        <li><b>A redirect weakened the link.</b> Ask for the link to point directly at the final URL, so no value is lost in the hop.</li>
      </ol>
      <div class="alt-box">
        <p><b>LinkPulz monitors the status of your backlinks and their destinations.</b> <span class="tc">TO CONFIRM that both are checked</span></p>
        <p>You get an alert when either side starts returning an error, so you can fix it while it still matters instead of discovering it in a quarterly audit.</p>
        <a class="btn" href="/register">Monitor my backlinks</a>
      </div>
    </div>
  </div>
</section>

<!-- RELATED -->
<section aria-labelledby="vs-h">
  <div class="wrap">
    <div class="prose">
      <h2 id="vs-h">Broken, lost or changed?</h2>
      <p>Problem links come in a few shapes, and each has its own fix:</p>
      <ul class="check">
        <li><b>Broken</b>, the link errors, or sits on a page that errors. Covered here.</li>
        <li><b>Lost</b>, the link has been removed from the page. See <a href="/lost-backlinks">lost backlinks</a>.</li>
        <li><b>Changed</b>, the anchor, target or rel attribute changed. See the <a href="/backlink-health-check">backlink health check</a>.</li>
      </ul>
    </div>
  </div>
</section>

<!-- FAQ -->
<section class="alt" aria-labelledby="faq-h">
  <div class="wrap">
    <div class="center"><h2 id="faq-h">Broken backlinks: frequently asked questions</h2></div>
    <div class="faq">
      <details><summary>What is a broken backlink?</summary><p>A broken backlink is a link that no longer works as intended. Either the link points to a URL that returns an error (such as a 404), or the link sits on a page that itself returns an error, so the link is unreachable.</p></details>
      <details><summary>How are broken backlinks different from lost backlinks?</summary><p>A lost backlink has been removed from the page. A broken backlink is still there, but it points to an error or lives on a page that errors, so it no longer passes value or sends traffic.</p></details>
      <details><summary>How do I find broken backlinks?</summary><p>Check two things for each link: the HTTP status of the page that links to you, and the HTTP status of the URL it points to. A broken backlink checker fetches both and flags any that return an error or a redirect.</p></details>
      <details><summary>How do I fix a broken backlink?</summary><p>If the broken URL is on your own site, add a 301 redirect from the old URL to the right page so the link keeps its value. If the linking page itself is broken, contact the site owner to restore the page or update the link.</p></details>
      <details><summary>Do broken backlinks hurt SEO?</summary><p>A link that points to a 404 on your site wastes the authority it could have passed, and a dead linking page sends no traffic. Fixing broken links recovers value you already earned.</p></details>
      <details><summary>Can LinkPulz find broken backlinks automatically?</summary><p>Yes. LinkPulz monitors the status of your backlinks and their destinations and alerts you when either side starts returning an error. <span class="tc">TO CONFIRM that both are checked, and alert channels</span></p></details>
    </div>
  </div>
</section>

<!-- MORE TOOLS -->
<section aria-labelledby="more-h">
  <div class="wrap">
    <div class="center"><h2 id="more-h">More backlink tools and guides</h2></div>
    <div class="cards">
      <div class="card"><h3><a href="/verify-backlinks">Verify a Backlink</a></h3><p>Check whether a single backlink exists, is live and is set up correctly.</p></div>
      <div class="card"><h3><a href="/lost-backlinks">Lost Backlinks</a></h3><p>Find links that have disappeared, understand why, and win them back.</p></div>
      <div class="card"><h3><a href="/backlink-health-check">Backlink Health Check</a></h3><p>See the overall state of your backlink portfolio at a glance.</p></div>
    </div>
  </div>
</section>

<!-- FINAL CTA -->
<section class="final" aria-labelledby="final-h">
  <div class="wrap">
    <h2 id="final-h">Don't let a 404 drain a link you earned.</h2>
    <p class="lead">Broken backlinks pass no value and send no traffic. Monitor your links and their destinations with LinkPulz and fix errors while they still matter.</p>
    <div class="cta-row" style="justify-content:center"><a class="btn" href="/register">Find Broken Backlinks Automatically</a></div>
  </div>
</section>
`;

export default function BrokenBacklinksPage() {
  return <div dangerouslySetInnerHTML={{ __html: marketingPage(JSONLD, MAIN) }} />;
}
