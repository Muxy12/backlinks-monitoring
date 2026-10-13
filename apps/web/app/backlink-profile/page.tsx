import type { Metadata } from "next";

export const dynamic = "force-static"; // public content → prerender + edge cache (pairs with the static marketing CSP in middleware.ts)
import { marketingPage } from "../marketing";

// Backlink Profile — SEO guide/tool page on the shared marketing shell. Unique
// content targeting "backlink profile" / "what is a backlink profile" /
// "analyze backlink profile". Intent: understand and analyse the composite
// picture (dofollow ratio, anchors, referring domains), distinct from the
// one-off audit and ongoing management pages. TO CONFIRM markers flag claims.
export const metadata: Metadata = {
  title: { absolute: "Backlink Profile: What It Is & How to Analyze It | LinkPulz" },
  description:
    "What a backlink profile is and how to analyze yours: referring domains, dofollow ratio, anchor text and relevance, plus how to keep the profile healthy over time.",
  alternates: { canonical: "/backlink-profile", languages: { en: "/backlink-profile", "x-default": "/backlink-profile" } },
  robots: { index: true, follow: true, "max-image-preview": "large" },
  openGraph: {
    siteName: "LinkPulz",
    type: "website",
    title: "Backlink Profile: What It Is & How to Analyze It | LinkPulz",
    description:
      "Understand your backlink profile: referring domains, dofollow ratio, anchor text and relevance, and how to keep it healthy.",
    url: "/backlink-profile",
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
      "@id": "https://www.linkpulz.com/backlink-profile#webpage",
      "url": "https://www.linkpulz.com/backlink-profile",
      "name": "Backlink Profile: What It Is & How to Analyze It | LinkPulz",
      "description": "What a backlink profile is and how to analyze yours: referring domains, dofollow ratio, anchor text and relevance, plus how to keep the profile healthy over time.",
      "isPartOf": { "@id": "https://www.linkpulz.com/#website" },
      "about": { "@id": "https://www.linkpulz.com/#software" },
      "breadcrumb": { "@id": "https://www.linkpulz.com/backlink-profile#breadcrumb" },
      "inLanguage": "en"
    },
    {
      "@type": "BreadcrumbList",
      "@id": "https://www.linkpulz.com/backlink-profile#breadcrumb",
      "itemListElement": [
        { "@type": "ListItem", "position": 1, "name": "Home", "item": "https://www.linkpulz.com/" },
        { "@type": "ListItem", "position": 2, "name": "Backlink Profile", "item": "https://www.linkpulz.com/backlink-profile" }
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
      "@id": "https://www.linkpulz.com/backlink-profile#faq",
      "mainEntity": [
        { "@type": "Question", "name": "What is a backlink profile?", "acceptedAnswer": { "@type": "Answer", "text": "A backlink profile is the complete set of links pointing to your site, taken together with their characteristics: which domains they come from, whether they are dofollow or nofollow, their anchor text, and how relevant and authoritative the linking sites are." } },
        { "@type": "Question", "name": "What does a healthy backlink profile look like?", "acceptedAnswer": { "@type": "Answer", "text": "A healthy profile tends to have links from a range of relevant, trustworthy domains, a natural mix of dofollow and nofollow, varied anchor text rather than the same exact phrase repeated, and steady growth rather than sudden unexplained spikes." } },
        { "@type": "Question", "name": "How do I analyze my backlink profile?", "acceptedAnswer": { "@type": "Answer", "text": "Look at the number of referring domains, the dofollow to nofollow ratio, the distribution of anchor text, the relevance and quality of the linking sites, and how the profile is changing over time, including links you have lost." } },
        { "@type": "Question", "name": "What is the dofollow to nofollow ratio?", "acceptedAnswer": { "@type": "Answer", "text": "It is the share of your links that pass authority (dofollow) versus those that do not (nofollow). A natural profile contains both. There is no single correct number, but a sudden swing toward nofollow can mean links you valued have quietly changed." } },
        { "@type": "Question", "name": "Why does anchor text distribution matter?", "acceptedAnswer": { "@type": "Answer", "text": "Anchor text tells search engines what a link is about. A natural profile has varied anchors, brand, URL, and descriptive phrases. An unnatural concentration of the same exact-match keyword can look manipulative." } },
        { "@type": "Question", "name": "How does LinkPulz help with my backlink profile?", "acceptedAnswer": { "@type": "Answer", "text": "LinkPulz focuses on keeping the profile accurate over time: it re-checks each link you add and alerts you when one is lost, redirected, nofollowed, noindexed or broken, so your view of the profile reflects reality rather than the day you last looked." } }
      ]
    }
  ]
}`;

const MAIN = `
<div class="wrap">
  <nav class="crumbs" aria-label="Breadcrumb"><a href="/">Home</a><span>›</span>Backlink Profile</nav>
</div>

<!-- HERO -->
<section class="hero" aria-labelledby="h1">
  <div class="wrap grid">
    <div>
      <span class="badge">Backlink profile</span>
      <h1 id="h1">Backlink Profile: The Full Picture of Your Links</h1>
      <p class="lead">Your backlink profile is every link pointing to your site, seen together, the domains they come from, the dofollow mix, the anchor text, the quality. Learn what a healthy profile looks like, how to analyze yours, and how to keep it accurate over time.</p>
      <div class="cta-row">
        <a class="btn" href="/register">Monitor my backlink profile</a>
        <a class="btn ghost" href="#analyze">How to analyze it</a>
      </div>
      <p class="micro">No credit card required</p>
    </div>
    <div class="health" role="img" aria-label="Example backlink profile snapshot with a health score and a breakdown of link states" style="margin:0">
      <div class="health-top">
        <div class="health-score"><small>Profile score</small><b>82</b><span>/ 100</span></div>
        <div class="health-main">
          <div class="health-bar"><i style="width:74%;background:var(--accent)"></i><i style="width:12%;background:var(--warn)"></i><i style="width:14%;background:var(--danger)"></i></div>
          <table class="health-table">
            <tbody>
              <tr><th><span class="dot" style="background:var(--accent)"></span>Dofollow &amp; live</th><td>74%</td></tr>
              <tr><th><span class="dot" style="background:var(--warn)"></span>Nofollow</th><td>12%</td></tr>
              <tr><th><span class="dot" style="background:var(--danger)"></span>Lost or broken</th><td>14%</td></tr>
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
      <h2 id="what-h">What is a backlink profile?</h2>
      <p>A backlink profile is the complete set of links pointing to your site, considered together with their characteristics, not just how many links you have, but where they come from and what kind of links they are.</p>
      <p>Where a single <a href="/verify-backlinks">backlink check</a> tells you about one link, the profile is the aggregate view: the shape of all your links at once. It is the thing a <a href="/backlink-audit">backlink audit</a> examines and that <a href="/backlink-management">backlink management</a> keeps healthy.</p>
    </div>
  </div>
</section>

<!-- WHAT MAKES IT HEALTHY -->
<section aria-labelledby="healthy-h">
  <div class="wrap">
    <div class="prose">
      <h2 id="healthy-h">What makes a backlink profile healthy</h2>
      <p>There is no single magic number, but healthy profiles tend to share a few traits:</p>
      <ul class="check">
        <li><b>A range of referring domains.</b> Links from many relevant sites beat many links from one site.</li>
        <li><b>Relevance and quality.</b> Links from trustworthy sites in your field count for more than volume alone.</li>
        <li><b>A natural dofollow / nofollow mix.</b> Real profiles contain both; an all-dofollow profile looks engineered.</li>
        <li><b>Varied anchor text.</b> Brand, bare URL and descriptive anchors, not the same exact phrase over and over.</li>
        <li><b>Steady change over time.</b> Gradual growth reads as organic; sudden unexplained spikes do not.</li>
        <li><b>Few lost or broken links.</b> A profile leaking links is one nobody is maintaining.</li>
      </ul>
    </div>
  </div>
</section>

<!-- HOW TO ANALYZE -->
<section class="alt" id="analyze" aria-labelledby="analyze-h">
  <div class="wrap">
    <div class="prose">
      <h2 id="analyze-h">How to analyze your backlink profile</h2>
      <ol>
        <li><b>Count your referring domains.</b> Unique linking domains matter more than raw link count.</li>
        <li><b>Check the dofollow / nofollow ratio.</b> Note any sharp swing toward nofollow, it can mean valued links quietly changed.</li>
        <li><b>Map the anchor text.</b> Look for healthy variety, and for any over-concentration of exact-match keywords.</li>
        <li><b>Judge relevance and quality.</b> Are the linking sites in your field and worth being associated with?</li>
        <li><b>Track change over time.</b> New links, and especially <a href="/lost-backlinks">lost links</a>, tell you where the profile is heading.</li>
      </ol>
    </div>
  </div>
</section>

<!-- KEEPING IT ACCURATE -->
<section aria-labelledby="keep-h">
  <div class="wrap">
    <div class="prose">
      <h2 id="keep-h">A profile is only as good as its data</h2>
      <p>The hard part is not analyzing the profile once, it is keeping that analysis true. A profile you checked last quarter already describes a site that no longer exists: links have been removed, nofollowed or broken since.</p>
      <p>LinkPulz focuses on exactly that gap. It re-checks each link you add and alerts you when one is lost, redirected, nofollowed, noindexed or broken, so your view of the profile reflects reality rather than the last time you looked.</p>
      <p>For a single-screen summary of where the profile stands, see the <a href="/backlink-health-check">backlink health check</a>.</p>
    </div>
  </div>
</section>

<!-- FAQ -->
<section class="alt" aria-labelledby="faq-h">
  <div class="wrap">
    <div class="center"><h2 id="faq-h">Backlink profile: frequently asked questions</h2></div>
    <div class="faq">
      <details><summary>What is a backlink profile?</summary><p>A backlink profile is the complete set of links pointing to your site, taken together with their characteristics: which domains they come from, whether they are dofollow or nofollow, their anchor text, and how relevant and authoritative the linking sites are.</p></details>
      <details><summary>What does a healthy backlink profile look like?</summary><p>A healthy profile tends to have links from a range of relevant, trustworthy domains, a natural mix of dofollow and nofollow, varied anchor text rather than the same exact phrase repeated, and steady growth rather than sudden unexplained spikes.</p></details>
      <details><summary>How do I analyze my backlink profile?</summary><p>Look at the number of referring domains, the dofollow to nofollow ratio, the distribution of anchor text, the relevance and quality of the linking sites, and how the profile is changing over time, including links you have lost.</p></details>
      <details><summary>What is the dofollow to nofollow ratio?</summary><p>It is the share of your links that pass authority (dofollow) versus those that do not (nofollow). A natural profile contains both. There is no single correct number, but a sudden swing toward nofollow can mean links you valued have quietly changed.</p></details>
      <details><summary>Why does anchor text distribution matter?</summary><p>Anchor text tells search engines what a link is about. A natural profile has varied anchors, brand, URL, and descriptive phrases. An unnatural concentration of the same exact-match keyword can look manipulative.</p></details>
      <details><summary>How does LinkPulz help with my backlink profile?</summary><p>LinkPulz focuses on keeping the profile accurate over time: it re-checks each link you add and alerts you when one is lost, redirected, nofollowed, noindexed or broken, so your view of the profile reflects reality rather than the day you last looked.</p></details>
    </div>
  </div>
</section>

<!-- MORE TOOLS -->
<section aria-labelledby="more-h">
  <div class="wrap">
    <div class="center"><h2 id="more-h">More backlink tools and guides</h2></div>
    <div class="cards">
      <div class="card"><h3><a href="/backlink-audit">Backlink Audit</a></h3><p>Run a point-in-time review to find the problems in your link profile.</p></div>
      <div class="card"><h3><a href="/backlink-health-check">Backlink Health Check</a></h3><p>See the overall state of your backlink portfolio at a glance.</p></div>
      <div class="card"><h3><a href="/backlink-management">Backlink Management</a></h3><p>Keep every link in one place and monitor it over time.</p></div>
    </div>
  </div>
</section>

<!-- FINAL CTA -->
<section class="final" aria-labelledby="final-h">
  <div class="wrap">
    <h2 id="final-h">Know your profile, not last quarter's.</h2>
    <p class="lead">Put your links under monitoring with LinkPulz so your backlink profile always reflects what's live today, and you're alerted the moment the picture changes.</p>
    <div class="cta-row" style="justify-content:center"><a class="btn" href="/register">Monitor my backlink profile</a></div>
  </div>
</section>
`;

export default function BacklinkProfilePage() {
  return <div dangerouslySetInnerHTML={{ __html: marketingPage(JSONLD, MAIN) }} />;
}
