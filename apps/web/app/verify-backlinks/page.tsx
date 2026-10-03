import type { Metadata } from "next";
import { marketingPage } from "../marketing";

// Verify Backlinks — SEO tool page. Content kept verbatim (copy, example results,
// comparison table, FAQ, JSON-LD, TO CONFIRM placeholders). Built on the shared
// marketing shell (./marketing.ts) so the charte, header and footer match the
// homepage. Head SEO goes through Next metadata; the JSON-LD graph stays inline.
export const metadata: Metadata = {
  title: { absolute: "Verify Backlinks: Backlink Verification Tool | LinkPulz" },
  description:
    "Check if a backlink exists, is live and is dofollow. Use our backlink verification tool, then monitor the link automatically with LinkPulz.",
  alternates: { canonical: "/verify-backlinks" },
  robots: { index: true, follow: true, "max-image-preview": "large" },
  openGraph: {
    type: "website",
    title: "Verify Backlinks: Backlink Verification Tool | LinkPulz",
    description:
      "Check if a backlink exists, is live and is dofollow. Use our backlink verification tool, then monitor the link automatically with LinkPulz.",
    url: "/verify-backlinks",
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
      "logo": {
        "@type": "ImageObject",
        "url": "https://ye8jhaekbbcviws9.public.blob.vercel-storage.com/logo-linkpulz.webp"
      },
      "sameAs": [
        "https://www.linkedin.com/company/linkpulz",
        "https://x.com/linkpulz"
      ]
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
      "@id": "https://www.linkpulz.com/verify-backlinks#webpage",
      "url": "https://www.linkpulz.com/verify-backlinks",
      "name": "Verify Backlinks: Backlink Verification Tool | LinkPulz",
      "description": "Check if a backlink exists, is live and is dofollow. Use our backlink verification tool, then monitor the link automatically with LinkPulz.",
      "isPartOf": { "@id": "https://www.linkpulz.com/#website" },
      "about": { "@id": "https://www.linkpulz.com/#software" },
      "breadcrumb": { "@id": "https://www.linkpulz.com/verify-backlinks#breadcrumb" },
      "inLanguage": "en"
    },
    {
      "@type": "BreadcrumbList",
      "@id": "https://www.linkpulz.com/verify-backlinks#breadcrumb",
      "itemListElement": [
        { "@type": "ListItem", "position": 1, "name": "Home", "item": "https://www.linkpulz.com/" },
        { "@type": "ListItem", "position": 2, "name": "Verify Backlinks", "item": "https://www.linkpulz.com/verify-backlinks" }
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
      "offers": {
        "@type": "Offer",
        "price": "0",
        "priceCurrency": "USD",
        "description": "Free trial. TO CONFIRM: replace with your real starting plan, price and currency.",
        "url": "https://www.linkpulz.com/#pricing"
      }
    },
    {
      "@type": "WebApplication",
      "@id": "https://www.linkpulz.com/verify-backlinks#tool",
      "name": "LinkPulz Backlink Verification Tool",
      "url": "https://www.linkpulz.com/verify-backlinks",
      "applicationCategory": "BusinessApplication",
      "operatingSystem": "Web",
      "description": "Check whether a backlink exists on a page, its HTTP status, link attributes, anchor text and target URL.",
      "publisher": { "@id": "https://www.linkpulz.com/#organization" },
      "offers": {
        "@type": "Offer",
        "price": "0",
        "priceCurrency": "USD"
      }
    },
    {
      "@type": "FAQPage",
      "@id": "https://www.linkpulz.com/verify-backlinks#faq",
      "mainEntity": [
        {
          "@type": "Question",
          "name": "How do I check if a backlink exists?",
          "acceptedAnswer": { "@type": "Answer", "text": "Open the source page, view its HTML source and search for your domain. Or paste the source URL and your URL into a backlink verification tool, which checks the page for you and reports whether the link is present." }
        },
        {
          "@type": "Question",
          "name": "What is a backlink verification tool?",
          "acceptedAnswer": { "@type": "Answer", "text": "A backlink verification tool loads the page that should contain your link and checks whether the link is there, what status the page returns, how the link is attributed (follow, nofollow, sponsored or UGC), what its anchor text is, and which URL it points to." }
        },
        {
          "@type": "Question",
          "name": "How can I tell if a backlink is dofollow or nofollow?",
          "acceptedAnswer": { "@type": "Answer", "text": "Look at the link's rel attribute in the page source. A link without nofollow, sponsored or ugc is treated as a normal followed link. A verification tool reads this attribute for you." }
        },
        {
          "@type": "Question",
          "name": "Does a backlink that exists always count?",
          "acceptedAnswer": { "@type": "Answer", "text": "Not necessarily. Search engines treat links as signals, not guarantees, and a link on a page that is noindex, low quality or not crawled may pass little value. Verifying only confirms that the link is present and correctly set up." }
        },
        {
          "@type": "Question",
          "name": "How often should I verify my backlinks?",
          "acceptedAnswer": { "@type": "Answer", "text": "Right after a link is published, then regularly, because links can be removed or changed at any time. Monitoring automates the repeat checks so you do not have to verify manually. [TO CONFIRM: LinkPulz check frequency]" }
        },
        {
          "@type": "Question",
          "name": "Can LinkPulz verify backlinks automatically?",
          "acceptedAnswer": { "@type": "Answer", "text": "Yes. Add your links to LinkPulz and it re-checks them on a schedule, then alerts you when a link is lost, redirected, broken or changes attributes. [TO CONFIRM]" }
        }
      ]
    }
  ]
}`;

const MAIN = `
<div class="wrap">
  <nav class="crumbs" aria-label="Breadcrumb"><a href="/">Home</a><span>›</span>Verify Backlinks</nav>
</div>

<!-- ============ HERO + TOOL ============ -->
<section aria-labelledby="h1">
  <div class="wrap center">
    <span class="badge">Backlink verification tool</span>
    <h1 id="h1">Verify Backlinks: Check If a Backlink Still Exists</h1>
    <p class="lead">Paste the page that links to you and your own URL. Find out in seconds whether the backlink is live, how it is set up, and whether it points to the right page.</p>

    <div class="tool verify">
      <form action="/verify-backlinks" method="get">
        <label>Source URL
          <input type="url" name="source" placeholder="https://example.com/article" required>
        </label>
        <label>Your URL
          <input type="url" name="target" placeholder="https://mysite.com" required>
        </label>
        <button class="btn" type="submit">Verify backlink</button>
      </form>

      <div class="verify-result" aria-live="polite">
        <p class="micro verify-label">Example result</p>
        <ul class="check">
          <li><b>Backlink found</b></li>
          <li>HTTP 200</li>
          <li>Dofollow</li>
          <li>Anchor: "SEO tool"</li>
          <li>Target URL correct</li>
        </ul>
      </div>

      <div class="verify-cta">
        <p><b>Want us to check it automatically every day?</b> <span class="tc">frequency: TO CONFIRM</span></p>
        <a class="btn" href="/register">Monitor this backlink with LinkPulz →</a>
      </div>
    </div>
  </div>
</section>

<!-- ============ WHAT IT CHECKS ============ -->
<section class="alt" aria-labelledby="checks-h">
  <div class="wrap">
    <div class="center">
      <h2 id="checks-h">What our backlink verification tool checks</h2>
      <p class="lead">One check tells you whether the link exists and whether it is set up the way you agreed.</p>
    </div>
    <div class="cards">
      <div class="card"><div class="ic">✓</div><h3>Backlink found</h3><p>Is a link to your URL present in the HTML of the source page?</p></div>
      <div class="card"><div class="ic">200</div><h3>HTTP status</h3><p>Does the source page load normally (200), redirect, or return an error?</p></div>
      <div class="card"><div class="ic">rel</div><h3>Link attributes</h3><p>Is the link followed, or marked nofollow, sponsored or UGC?</p></div>
      <div class="card"><div class="ic">Aa</div><h3>Anchor text</h3><p>What text is the link using? Does it match what was agreed?</p></div>
      <div class="card"><div class="ic">→</div><h3>Target URL</h3><p>Does the link point to the exact page you expect, not an old or wrong URL?</p></div>
      <div class="card"><div class="ic">NI</div><h3>Indexing signals</h3><p>Does the source page carry a noindex directive? <span class="tc">TO CONFIRM</span></p></div>
    </div>
  </div>
</section>

<!-- ============ WHY VERIFY ============ -->
<section aria-labelledby="why-h">
  <div class="wrap">
    <div class="prose">
      <h2 id="why-h">Why verify a backlink?</h2>
      <p>A backlink is only worth something if it is actually there, working and pointing at the right page. Verifying backlinks is the quickest way to find out. It is useful when:</p>
      <ul class="check">
        <li><b>You just earned a link.</b> After outreach, a guest post or a digital PR campaign, confirm the link was published correctly.</li>
        <li><b>You paid for a link.</b> Check it is live and carries the attributes you agreed on, for example no unexpected nofollow.</li>
        <li><b>You are reporting to a client.</b> Prove that the links you delivered exist before you put them in a report.</li>
        <li><b>The publisher updated the page.</b> Content edits, redesigns and migrations can quietly remove or change links.</li>
        <li><b>Something looks off.</b> Referral traffic from a link dropped, or a link no longer shows in your tools.</li>
      </ul>
    </div>
  </div>
</section>

<!-- ============ HOW MANUALLY ============ -->
<section class="alt" aria-labelledby="manual-h">
  <div class="wrap">
    <div class="prose">
      <h2 id="manual-h">How to check if a backlink exists manually</h2>
      <p>You can verify a single backlink without any tool:</p>
      <ol>
        <li><b>Open the source page</b> in your browser and look for your link. Note the text it uses.</li>
        <li><b>View the page source.</b> Right-click and choose "View page source", or press Ctrl+U (Cmd+Option+U on Mac). Search for your domain name.</li>
        <li><b>Read the link's rel attribute.</b> Values like nofollow, sponsored or ugc tell search engines how to treat the link.</li>
        <li><b>Check the href.</b> Make sure it points to the exact URL you expect, with no typo and no outdated address.</li>
        <li><b>Look for a robots meta tag.</b> A noindex in the head of the source page is worth knowing about.</li>
      </ol>
      <p>That works well for one link. For dozens or hundreds, it does not scale, and it only tells you the state of the link today.</p>
    </div>
  </div>
</section>

<!-- ============ VERIFICATION VS MONITORING ============ -->
<section aria-labelledby="vs-h">
  <div class="wrap">
    <div class="center">
      <h2 id="vs-h">Backlink verification vs. backlink monitoring</h2>
      <p class="lead">Verification answers "is it there right now?". Monitoring answers "is it still there tomorrow?".</p>
    </div>
    <div class="tablewrap">
      <table>
        <thead><tr><th></th><th>Backlink verification</th><th>Backlink monitoring with LinkPulz</th></tr></thead>
        <tbody>
          <tr><th>When it runs</th><td>Once, when you check</td><td>On a schedule <span class="tc">frequency: TO CONFIRM</span></td></tr>
          <tr><th>What you learn</th><td>The link's state at that moment</td><td>The link's state and every change over time</td></tr>
          <tr><th>If the link disappears</th><td>You only find out if you check again</td><td>You get an alert <span class="tc">channels: TO CONFIRM</span></td></tr>
          <tr><th>Best for</th><td>Confirming a new link</td><td>Protecting links you earned or paid for</td></tr>
        </tbody>
      </table>
    </div>
    <div class="center" style="margin-top:32px">
      <a class="btn" href="/register">Start monitoring your backlinks</a>
      <p class="micro" style="margin-top:12px">Links that disappear? See <a href="/lost-backlinks">lost backlinks</a>. Links that lead to errors? See <a href="/broken-backlinks">broken backlinks</a>.</p>
    </div>
  </div>
</section>

<!-- ============ FAQ ============ -->
<section class="alt" aria-labelledby="faq-h">
  <div class="wrap">
    <div class="center"><h2 id="faq-h">Verify backlinks: frequently asked questions</h2></div>
    <div class="faq">
      <details><summary>How do I check if a backlink exists?</summary><p>Open the source page, view its HTML source and search for your domain. Or paste the source URL and your URL into a backlink verification tool, which checks the page for you and reports whether the link is present.</p></details>
      <details><summary>What is a backlink verification tool?</summary><p>A backlink verification tool loads the page that should contain your link and checks whether the link is there, what status the page returns, how the link is attributed (follow, nofollow, sponsored or UGC), what its anchor text is, and which URL it points to.</p></details>
      <details><summary>How can I tell if a backlink is dofollow or nofollow?</summary><p>Look at the link's rel attribute in the page source. A link without nofollow, sponsored or ugc is treated as a normal followed link. A verification tool reads this attribute for you.</p></details>
      <details><summary>Does a backlink that exists always count?</summary><p>Not necessarily. Search engines treat links as signals, not guarantees, and a link on a page that is noindex, low quality or not crawled may pass little value. Verifying only confirms that the link is present and correctly set up.</p></details>
      <details><summary>How often should I verify my backlinks?</summary><p>Right after a link is published, then regularly, because links can be removed or changed at any time. Monitoring automates the repeat checks so you do not have to verify manually. <span class="tc">TO CONFIRM: LinkPulz check frequency</span></p></details>
      <details><summary>Can LinkPulz verify backlinks automatically?</summary><p>Yes. Add your links to LinkPulz and it re-checks them on a schedule, then alerts you when a link is lost, redirected, broken or changes attributes. <span class="tc">TO CONFIRM</span></p></details>
    </div>
  </div>
</section>

<!-- ============ MORE TOOLS ============ -->
<section aria-labelledby="more-h">
  <div class="wrap">
    <div class="center"><h2 id="more-h">More backlink tools and guides</h2></div>
    <div class="cards">
      <div class="card"><h3><a href="/lost-backlinks">Lost Backlinks</a></h3><p>Find links that have disappeared, understand why, and win them back.</p></div>
      <div class="card"><h3><a href="/broken-backlinks">Broken Backlinks</a></h3><p>Find links that lead to errors, or that sit on pages returning errors.</p></div>
      <div class="card"><h3><a href="/backlink-health-check">Backlink Health Check</a></h3><p>See the overall state of your backlink portfolio at a glance.</p></div>
    </div>
  </div>
</section>

<!-- ============ FINAL CTA ============ -->
<section class="final" aria-labelledby="final-h">
  <div class="wrap">
    <h2 id="final-h">Don't just verify a backlink once. Keep it verified.</h2>
    <p class="lead">Links change after you check them. Get alerted the moment one stops working. LinkPulz is a backlink monitoring tool built to do it for you.</p>
    <div class="cta-row" style="justify-content:center"><a class="btn" href="/register">Monitor my backlinks</a></div>
  </div>
</section>
`;

export default function VerifyBacklinksPage() {
  return <div dangerouslySetInnerHTML={{ __html: marketingPage(JSONLD, MAIN) }} />;
}
