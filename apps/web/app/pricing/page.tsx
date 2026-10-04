import type { Metadata } from "next";
import { marketingPage } from "../marketing";

// Pricing — public marketing page. Built on the shared marketing shell
// (../marketing.ts) so the charte, header and footer match the rest of the site.
// The monthly/annual toggle is pure CSS (hidden radios + `:checked ~` selectors),
// so the page stays a zero-JS, fully server-rendered marketing page — best for SEO.
//
// Prices, limits and the 48h interval MIRROR the single source of truth,
// crates/billing/src/plans.rs (served live at GET /v1/plans). When a price or
// limit changes there, update this page to match. The in-app billing page
// (settings/billing) reads /v1/plans directly; this public page is static for SEO
// and resilience. Annual = 10× monthly (2 months free): 29→290, 59→590, 99→990,
// 199→1990; "/mo" shown on annual is the yearly total ÷ 12, rounded.
export const metadata: Metadata = {
  title: { absolute: "Pricing: Backlink Monitoring Plans & Free Trial | LinkPulz" },
  description:
    "Simple backlink monitoring pricing. Every plan includes a 10-day free trial (no credit card) and checks every backlink every 48 hours. Starter $29, Pro $59, Business $99, Agency $199 per month.",
  alternates: { canonical: "/pricing" },
  robots: { index: true, follow: true, "max-image-preview": "large" },
  openGraph: {
    type: "website",
    title: "Pricing: Backlink Monitoring Plans & Free Trial | LinkPulz",
    description:
      "Simple backlink monitoring pricing. 10-day free trial, no credit card. Checks every 48 hours. Plans from $29/mo.",
    url: "/pricing",
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
      "@id": "https://www.linkpulz.com/pricing#webpage",
      "url": "https://www.linkpulz.com/pricing",
      "name": "Pricing: Backlink Monitoring Plans & Free Trial | LinkPulz",
      "description": "Backlink monitoring pricing with a 10-day free trial and checks every 48 hours. Plans from $29/mo.",
      "isPartOf": { "@id": "https://www.linkpulz.com/#website" },
      "about": { "@id": "https://www.linkpulz.com/#software" },
      "breadcrumb": { "@id": "https://www.linkpulz.com/pricing#breadcrumb" },
      "inLanguage": "en"
    },
    {
      "@type": "BreadcrumbList",
      "@id": "https://www.linkpulz.com/pricing#breadcrumb",
      "itemListElement": [
        { "@type": "ListItem", "position": 1, "name": "Home", "item": "https://www.linkpulz.com/" },
        { "@type": "ListItem", "position": 2, "name": "Pricing", "item": "https://www.linkpulz.com/pricing" }
      ]
    },
    {
      "@type": "SoftwareApplication",
      "@id": "https://www.linkpulz.com/#software",
      "name": "LinkPulz",
      "applicationCategory": "BusinessApplication",
      "applicationSubCategory": "SEO software",
      "operatingSystem": "Web",
      "description": "Backlink monitoring software that checks your earned and paid backlinks every 48 hours and alerts you when a link is lost, redirected, nofollowed, noindexed or broken.",
      "url": "https://www.linkpulz.com/",
      "publisher": { "@id": "https://www.linkpulz.com/#organization" },
      "offers": {
        "@type": "AggregateOffer",
        "priceCurrency": "USD",
        "lowPrice": "29",
        "highPrice": "199",
        "offerCount": "4",
        "url": "https://www.linkpulz.com/pricing",
        "offers": [
          { "@type": "Offer", "name": "Starter", "price": "29", "priceCurrency": "USD" },
          { "@type": "Offer", "name": "Pro", "price": "59", "priceCurrency": "USD" },
          { "@type": "Offer", "name": "Business", "price": "99", "priceCurrency": "USD" },
          { "@type": "Offer", "name": "Agency", "price": "199", "priceCurrency": "USD" }
        ]
      }
    },
    {
      "@type": "FAQPage",
      "@id": "https://www.linkpulz.com/pricing#faq",
      "mainEntity": [
        {
          "@type": "Question",
          "name": "Is there a free trial?",
          "acceptedAnswer": { "@type": "Answer", "text": "Yes. Every plan starts with a 10-day free trial and no credit card is required. You can monitor backlinks right away and only choose a plan when the trial ends." }
        },
        {
          "@type": "Question",
          "name": "Do I need a credit card to start?",
          "acceptedAnswer": { "@type": "Answer", "text": "No. You can start the 10-day trial with just an email. We ask for payment details only if you decide to continue on a paid plan." }
        },
        {
          "@type": "Question",
          "name": "What happens when my trial ends?",
          "acceptedAnswer": { "@type": "Answer", "text": "Nothing is deleted. Monitoring pauses and you choose a plan to continue. All your backlinks, projects and history are preserved and resume as soon as you subscribe." }
        },
        {
          "@type": "Question",
          "name": "How often do you check my backlinks?",
          "acceptedAnswer": { "@type": "Answer", "text": "Every 48 hours, on every plan. LinkPulz re-checks each backlink on that schedule and alerts you when a link is lost, redirected, broken, nofollowed or noindexed." }
        },
        {
          "@type": "Question",
          "name": "What happens if I reach my plan limit?",
          "acceptedAnswer": { "@type": "Answer", "text": "You can keep monitoring everything you already track, going over a limit never stops monitoring or deletes data. You simply can't add new backlinks, projects or team members until you upgrade." }
        },
        {
          "@type": "Question",
          "name": "Can I change plans later?",
          "acceptedAnswer": { "@type": "Answer", "text": "Yes. You can upgrade or downgrade at any time from your billing settings. Upgrades take effect immediately so you can add more backlinks and projects straight away." }
        },
        {
          "@type": "Question",
          "name": "How does annual billing save money?",
          "acceptedAnswer": { "@type": "Answer", "text": "Annual plans are billed once a year at the price of ten months, so you get two months free compared with paying monthly." }
        },
        {
          "@type": "Question",
          "name": "Can I cancel anytime?",
          "acceptedAnswer": { "@type": "Answer", "text": "Yes. You can cancel whenever you like. Your plan stays active until the end of the billing period you've already paid for." }
        }
      ]
    }
  ]
}`;

const MAIN = `
<div class="wrap">
  <nav class="crumbs" aria-label="Breadcrumb"><a href="/">Home</a><span>›</span>Pricing</nav>
</div>

<!-- ============ HERO + PLANS ============ -->
<section aria-labelledby="h1">
  <div class="wrap center">
    <span class="badge">Pricing</span>
    <h1 id="h1">Simple pricing for backlink monitoring</h1>
    <p class="lead">Every plan includes a <b>10-day free trial, no credit card</b>. Automatic checks every 48 hours, with an alert the moment a link changes.</p>

    <input type="radio" name="billing" id="bill-m" class="bill-radio" checked>
    <input type="radio" name="billing" id="bill-a" class="bill-radio">
    <div class="bill-toggle" role="group" aria-label="Billing period">
      <label for="bill-m" class="seg">Monthly</label>
      <label for="bill-a" class="seg">Annual <span class="save-badge">2 months free</span></label>
    </div>

    <div class="plans">

      <div class="plan">
        <h3>Starter</h3>
        <p class="pos">For website owners and small SEO projects.</p>
        <div class="price-block price-monthly">
          <div class="price">$29<small>/mo</small></div>
          <div class="per">billed monthly</div>
        </div>
        <div class="price-block price-annual">
          <div class="price">$24<small>/mo</small></div>
          <div class="per">$290 billed yearly</div>
        </div>
        <div class="limits">
          <span><b>500</b> backlinks monitored</span>
          <span><b>3</b> projects</span>
          <span><b>1</b> user</span>
        </div>
        <ul class="feats">
          <li>Checks every 48 hours</li>
          <li>Lost &amp; removed link alerts</li>
          <li>Broken &amp; redirected link detection</li>
          <li>Nofollow / noindex change alerts</li>
          <li>CSV import</li>
          <li>Email alerts</li>
        </ul>
        <a class="btn ghost" href="/register">Start free trial</a>
      </div>

      <div class="plan hl">
        <span class="pop">Most popular</span>
        <h3>Pro</h3>
        <p class="pos">For SEO professionals and link builders.</p>
        <div class="price-block price-monthly">
          <div class="price">$59<small>/mo</small></div>
          <div class="per">billed monthly</div>
        </div>
        <div class="price-block price-annual">
          <div class="price">$49<small>/mo</small></div>
          <div class="per">$590 billed yearly</div>
        </div>
        <div class="limits">
          <span><b>2,500</b> backlinks monitored</span>
          <span><b>10</b> projects</span>
          <span><b>3</b> users</span>
        </div>
        <ul class="feats">
          <li>Everything in Starter</li>
          <li>Webhook &amp; Slack alerts</li>
          <li>Full backlink change history</li>
          <li>Team access</li>
        </ul>
        <a class="btn" href="/register">Start free trial</a>
      </div>

      <div class="plan">
        <h3>Business</h3>
        <p class="pos">For SEO teams managing multiple websites.</p>
        <div class="price-block price-monthly">
          <div class="price">$99<small>/mo</small></div>
          <div class="per">billed monthly</div>
        </div>
        <div class="price-block price-annual">
          <div class="price">$82<small>/mo</small></div>
          <div class="per">$990 billed yearly</div>
        </div>
        <div class="limits">
          <span><b>5,000</b> backlinks monitored</span>
          <span><b>25</b> projects</span>
          <span><b>5</b> users</span>
        </div>
        <ul class="feats">
          <li>Everything in Pro</li>
          <li>Priority email support</li>
          <li>Higher monitoring limits</li>
        </ul>
        <a class="btn ghost" href="/register">Start free trial</a>
      </div>

      <div class="plan">
        <h3>Agency</h3>
        <p class="pos">For agencies managing backlinks across many clients.</p>
        <div class="price-block price-monthly">
          <div class="price">$199<small>/mo</small></div>
          <div class="per">billed monthly</div>
        </div>
        <div class="price-block price-annual">
          <div class="price">$166<small>/mo</small></div>
          <div class="per">$1,990 billed yearly</div>
        </div>
        <div class="limits">
          <span><b>15,000</b> backlinks monitored</span>
          <span><b>75</b> projects</span>
          <span><b>15</b> users</span>
        </div>
        <ul class="feats">
          <li>Everything in Business</li>
          <li>Dedicated priority support</li>
          <li>Built for client portfolios</li>
        </ul>
        <a class="btn ghost" href="/register">Start free trial</a>
      </div>

    </div>
    <p class="price-note">All plans check every backlink every 48 hours. Prices in USD. Going over a limit never stops monitoring or deletes data, you just can't add new links until you upgrade.</p>
  </div>
</section>

<!-- ============ VALUE STRIP ============ -->
<section class="alt" aria-labelledby="value-h">
  <div class="wrap">
    <div class="center">
      <h2 id="value-h">What every plan includes</h2>
      <p class="lead">No tiered-away essentials. Monitoring and alerting work the same on every plan, the plans differ by how much you track and how big your team is.</p>
    </div>
    <div class="cards">
      <div class="card"><div class="ic">48h</div><h3>Checks every 48 hours</h3><p>Every backlink is re-checked on a fixed schedule, on every plan, not just the expensive ones.</p></div>
      <div class="card"><div class="ic">🔔</div><h3>Change alerts</h3><p>Get alerted when a link is lost, redirected, broken, nofollowed or noindexed.</p></div>
      <div class="card"><div class="ic">10</div><h3>10-day free trial</h3><p>Start monitoring with no credit card. Decide once you've seen it work.</p></div>
      <div class="card"><div class="ic">↑↓</div><h3>Change plans anytime</h3><p>Upgrade or downgrade whenever you need to. Over-limit never deletes your data.</p></div>
    </div>
  </div>
</section>

<!-- ============ FAQ ============ -->
<section aria-labelledby="faq-h">
  <div class="wrap">
    <div class="center"><h2 id="faq-h">Pricing: frequently asked questions</h2></div>
    <div class="faq">
      <details><summary>Is there a free trial?</summary><p>Yes. Every plan starts with a 10-day free trial and no credit card is required. You can monitor backlinks right away and only choose a plan when the trial ends.</p></details>
      <details><summary>Do I need a credit card to start?</summary><p>No. You can start the 10-day trial with just an email. We ask for payment details only if you decide to continue on a paid plan.</p></details>
      <details><summary>What happens when my trial ends?</summary><p>Nothing is deleted. Monitoring pauses and you choose a plan to continue. All your backlinks, projects and history are preserved and resume as soon as you subscribe.</p></details>
      <details><summary>How often do you check my backlinks?</summary><p>Every 48 hours, on every plan. LinkPulz re-checks each backlink on that schedule and alerts you when a link is lost, redirected, broken, nofollowed or noindexed.</p></details>
      <details><summary>What happens if I reach my plan limit?</summary><p>You can keep monitoring everything you already track, going over a limit never stops monitoring or deletes data. You simply can't add new backlinks, projects or team members until you upgrade.</p></details>
      <details><summary>Can I change plans later?</summary><p>Yes. You can upgrade or downgrade at any time from your billing settings. Upgrades take effect immediately so you can add more backlinks and projects straight away.</p></details>
      <details><summary>How does annual billing save money?</summary><p>Annual plans are billed once a year at the price of ten months, so you get two months free compared with paying monthly.</p></details>
      <details><summary>Can I cancel anytime?</summary><p>Yes. You can cancel whenever you like. Your plan stays active until the end of the billing period you've already paid for.</p></details>
    </div>
  </div>
</section>

<!-- ============ FINAL CTA ============ -->
<section class="final" aria-labelledby="final-h">
  <div class="wrap">
    <h2 id="final-h">Start monitoring your backlinks in minutes</h2>
    <p class="lead">10-day free trial, no credit card. Add your links and LinkPulz watches them for you, every 48 hours.</p>
    <div class="cta-row" style="justify-content:center"><a class="btn" href="/register">Start your free trial</a></div>
  </div>
</section>
`;

export default function PricingPage() {
  return <div dangerouslySetInnerHTML={{ __html: marketingPage(JSONLD, MAIN) }} />;
}
