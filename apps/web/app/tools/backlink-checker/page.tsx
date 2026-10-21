import type { Metadata } from "next";
import { MARKETING_STYLE, MARKETING_HEADER, MARKETING_FOOTER } from "../../marketing";
import { BacklinkCheckerForm } from "./BacklinkCheckerForm";

// Public content + a free interactive tool. The shell, SEO copy, FAQ and JSON-LD
// are server-rendered (static, edge-cached, pairs with the static marketing CSP);
// only the checker form itself is a client component that calls the API from the
// browser. The visible FAQ is kept in lockstep with the FAQPage JSON-LD below.
export const dynamic = "force-static";

export const metadata: Metadata = {
  title: { absolute: "Free Backlink Checker: Check Any Backlink Online | LinkPulz" },
  description:
    "Free online backlink checker. Paste a source page and your URL to see instantly if the backlink is live, its anchor text, whether it is dofollow or nofollow, its rel attributes and whether the source page is indexable. No signup.",
  alternates: {
    canonical: "/tools/backlink-checker",
    languages: { en: "/tools/backlink-checker", "x-default": "/tools/backlink-checker" },
  },
  robots: { index: true, follow: true, "max-image-preview": "large" },
  openGraph: {
    siteName: "LinkPulz",
    type: "website",
    title: "Free Backlink Checker: Check Any Backlink Online | LinkPulz",
    description:
      "Paste a source page and your URL to instantly check if the backlink is live, its anchor text, dofollow or nofollow, rel attributes and whether the source page is indexable. Free, no signup.",
    url: "/tools/backlink-checker",
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
      "sameAs": ["https://www.linkedin.com/showcase/linkpulz", "https://x.com/linkpulz"]
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
      "@id": "https://www.linkpulz.com/tools/backlink-checker#webpage",
      "url": "https://www.linkpulz.com/tools/backlink-checker",
      "name": "Free Backlink Checker: Check Any Backlink Online | LinkPulz",
      "description": "A free online backlink checker. Paste a source page and your target URL to see whether the backlink is live, its anchor text, dofollow or nofollow, rel attributes and whether the source page is indexable.",
      "isPartOf": { "@id": "https://www.linkpulz.com/#website" },
      "about": { "@id": "https://www.linkpulz.com/#software" },
      "mainEntity": { "@id": "https://www.linkpulz.com/tools/backlink-checker#webapp" },
      "breadcrumb": { "@id": "https://www.linkpulz.com/tools/backlink-checker#breadcrumb" },
      "inLanguage": "en"
    },
    {
      "@type": "BreadcrumbList",
      "@id": "https://www.linkpulz.com/tools/backlink-checker#breadcrumb",
      "itemListElement": [
        { "@type": "ListItem", "position": 1, "name": "Home", "item": "https://www.linkpulz.com/" },
        { "@type": "ListItem", "position": 2, "name": "Free Backlink Checker", "item": "https://www.linkpulz.com/tools/backlink-checker" }
      ]
    },
    {
      "@type": "WebApplication",
      "@id": "https://www.linkpulz.com/tools/backlink-checker#webapp",
      "name": "Free Backlink Checker",
      "url": "https://www.linkpulz.com/tools/backlink-checker",
      "applicationCategory": "BusinessApplication",
      "applicationSubCategory": "SEO tool",
      "operatingSystem": "Web",
      "browserRequirements": "Requires a modern web browser",
      "description": "A free online backlink checker that fetches a source page and reports whether it links to your target URL, with the anchor text, dofollow or nofollow status, rel attributes and whether the source page is indexable.",
      "offers": { "@type": "Offer", "price": "0", "priceCurrency": "USD" },
      "publisher": { "@id": "https://www.linkpulz.com/#organization" },
      "isPartOf": { "@id": "https://www.linkpulz.com/#software" }
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
      "@id": "https://www.linkpulz.com/tools/backlink-checker#faq",
      "mainEntity": [
        { "@type": "Question", "name": "Is the backlink checker free?", "acceptedAnswer": { "@type": "Answer", "text": "Yes. The backlink checker is free and needs no signup. It checks one backlink at a time: a single source page against a single target URL. To track many backlinks automatically on a schedule, use LinkPulz monitoring." } },
        { "@type": "Question", "name": "How does the backlink checker work?", "acceptedAnswer": { "@type": "Answer", "text": "It fetches the source page you entered and looks for a link pointing to your target URL. If it finds one, it reports the anchor text, whether the link is dofollow or nofollow, its rel attributes and whether the source page is indexable." } },
        { "@type": "Question", "name": "What does dofollow or nofollow mean?", "acceptedAnswer": { "@type": "Answer", "text": "A dofollow link passes ranking signals to the page it points to. A nofollow link (rel=\\"nofollow\\", or the related sponsored and UGC values) tells search engines not to pass those signals. The checker shows which one applies and the exact rel attributes it found." } },
        { "@type": "Question", "name": "Why does it say \\"not found\\" when I can see the link?", "acceptedAnswer": { "@type": "Answer", "text": "The checker reads the HTML the server returns. A link can be missing from that HTML if it is injected later by JavaScript, shown only to some visitors or regions, or sits on a different URL than the one you entered. Make sure the source URL is the exact page that holds the link." } },
        { "@type": "Question", "name": "What is the difference between a backlink checker and backlink monitoring?", "acceptedAnswer": { "@type": "Answer", "text": "A backlink checker gives you a one-time snapshot of a single link right now. Backlink monitoring re-checks your links on a schedule, keeps their history and alerts you the moment one is lost, redirected, nofollowed or broken." } },
        { "@type": "Question", "name": "Can I check many backlinks at once?", "acceptedAnswer": { "@type": "Answer", "text": "The free checker verifies one link at a time. To check and monitor many backlinks on a schedule, with email, Slack and webhook alerts, add them to LinkPulz and start a free trial." } }
      ]
    }
  ]
}`;

export default function BacklinkCheckerPage() {
  return (
    <>
      <script type="application/ld+json" dangerouslySetInnerHTML={{ __html: JSONLD }} />
      <div id="lp-home">
        <div dangerouslySetInnerHTML={{ __html: MARKETING_STYLE + MARKETING_HEADER }} />
        <main>
          <div className="wrap">
            <nav className="crumbs" aria-label="Breadcrumb">
              <a href="/">Home</a>
              <span>›</span>Free Backlink Checker
            </nav>
          </div>

          <section className="hero" aria-labelledby="h1">
            <div className="wrap">
              <span className="badge">Free tool · no signup</span>
              <h1 id="h1">Free Backlink Checker</h1>
              <p className="lead">
                Check whether a page actually links to you. Paste the source page and your target URL,
                and the backlink checker fetches the page and tells you if the link is live, its anchor
                text, whether it is dofollow or nofollow, its rel attributes and whether the source page
                is indexable.
              </p>
            </div>
          </section>

          <section aria-labelledby="tool-h">
            <div className="wrap" style={{ maxWidth: 620 }}>
              <h2 id="tool-h" className="sr-only">
                Check a backlink
              </h2>
              <BacklinkCheckerForm />
            </div>
          </section>

          <section className="alt" aria-labelledby="what-h">
            <div className="wrap">
              <div className="center">
                <h2 id="what-h">What the backlink checker tells you</h2>
                <p className="lead">One request, everything you need to confirm a single placement.</p>
              </div>
              <ul className="check">
                <li>
                  <b>Link status</b>, whether the source page currently links to your target URL.
                </li>
                <li>
                  <b>Anchor text</b>, the exact clickable text used for the link.
                </li>
                <li>
                  <b>Dofollow or nofollow</b>, plus the raw rel attributes found (nofollow, sponsored, UGC).
                </li>
                <li>
                  <b>Source indexable</b>, whether the linking page is noindex, which affects how much the
                  link is worth.
                </li>
                <li>
                  <b>HTTP status</b>, so you can tell a missing link from a page that failed to load.
                </li>
              </ul>
            </div>
          </section>

          <section aria-labelledby="how-h">
            <div className="wrap">
              <div className="center">
                <h2 id="how-h">How to check a backlink</h2>
              </div>
              <div className="cards">
                <div className="card">
                  <h3>1. Enter the source page</h3>
                  <p>Paste the full URL of the page that should contain your link, not the homepage of the site.</p>
                </div>
                <div className="card">
                  <h3>2. Enter your target URL</h3>
                  <p>Paste the exact page on your site the link should point to. The checker matches the destination.</p>
                </div>
                <div className="card">
                  <h3>3. Read the result</h3>
                  <p>See in seconds whether the link is live, its anchor text, its dofollow or nofollow status and more.</p>
                </div>
              </div>
            </div>
          </section>

          <section className="alt" aria-labelledby="vs-h">
            <div className="wrap">
              <div className="center">
                <h2 id="vs-h">Backlink checker vs. backlink monitoring</h2>
                <p className="lead">
                  A checker is a one-time snapshot of a single link. Monitoring is the ongoing version:
                  LinkPulz re-checks all your backlinks on a schedule, keeps their history and alerts you the
                  moment one is lost, redirected, nofollowed or broken, so you are not checking links by hand.
                </p>
                <div className="cta-row" style={{ justifyContent: "center" }}>
                  <a className="btn" href="/register">
                    Start monitoring your backlinks
                  </a>
                  <a className="btn ghost" href="/lost-backlinks">
                    Learn about lost backlinks
                  </a>
                </div>
              </div>
            </div>
          </section>

          <section aria-labelledby="more-h">
            <div className="wrap">
              <div className="center">
                <h2 id="more-h">Related tools &amp; guides</h2>
              </div>
              <ul className="check">
                <li>
                  <a href="/verify-backlinks">Verify a backlink</a>, the full backlink verification workflow.
                </li>
                <li>
                  <a href="/lost-backlinks">Lost backlinks</a>, find and recover links that disappeared.
                </li>
                <li>
                  <a href="/broken-backlinks">Broken backlinks</a>, spot links to or on pages that return errors.
                </li>
                <li>
                  <a href="/backlink-health-check">Backlink health check</a>, see the health of your whole profile.
                </li>
                <li>
                  <a href="/">Backlink monitoring</a>, track every link you earn or buy, automatically.
                </li>
              </ul>
            </div>
          </section>

          <section className="alt" id="faq" aria-labelledby="faq-h">
            <div className="wrap">
              <div className="center">
                <h2 id="faq-h">Backlink checker FAQ</h2>
              </div>
              <div className="faq">
                <details>
                  <summary>Is the backlink checker free?</summary>
                  <p>
                    Yes. The backlink checker is free and needs no signup. It checks one backlink at a time: a
                    single source page against a single target URL. To track many backlinks automatically on a
                    schedule, use LinkPulz monitoring.
                  </p>
                </details>
                <details>
                  <summary>How does the backlink checker work?</summary>
                  <p>
                    It fetches the source page you entered and looks for a link pointing to your target URL. If
                    it finds one, it reports the anchor text, whether the link is dofollow or nofollow, its rel
                    attributes and whether the source page is indexable.
                  </p>
                </details>
                <details>
                  <summary>What does dofollow or nofollow mean?</summary>
                  <p>
                    A dofollow link passes ranking signals to the page it points to. A nofollow link
                    (rel=&quot;nofollow&quot;, or the related sponsored and UGC values) tells search engines not
                    to pass those signals. The checker shows which one applies and the exact rel attributes it
                    found.
                  </p>
                </details>
                <details>
                  <summary>Why does it say &quot;not found&quot; when I can see the link?</summary>
                  <p>
                    The checker reads the HTML the server returns. A link can be missing from that HTML if it is
                    injected later by JavaScript, shown only to some visitors or regions, or sits on a different
                    URL than the one you entered. Make sure the source URL is the exact page that holds the link.
                  </p>
                </details>
                <details>
                  <summary>What is the difference between a backlink checker and backlink monitoring?</summary>
                  <p>
                    A backlink checker gives you a one-time snapshot of a single link right now. Backlink
                    monitoring re-checks your links on a schedule, keeps their history and alerts you the moment
                    one is lost, redirected, nofollowed or broken.
                  </p>
                </details>
                <details>
                  <summary>Can I check many backlinks at once?</summary>
                  <p>
                    The free checker verifies one link at a time. To check and monitor many backlinks on a
                    schedule, with email, Slack and webhook alerts, add them to LinkPulz and start a free trial.
                  </p>
                </details>
              </div>
            </div>
          </section>

          <section className="final" aria-labelledby="final-h">
            <div className="wrap">
              <h2 id="final-h">Check once, then never check by hand again</h2>
              <p className="lead">
                The free checker confirms a single link today. LinkPulz watches all of them for you, on a
                schedule you set, and tells you the moment something changes.
              </p>
              <div className="cta-row" style={{ justifyContent: "center" }}>
                <a className="btn" href="/register">
                  Start your free trial
                </a>
              </div>
              <p className="micro" style={{ marginTop: 14, color: "#8b98b4" }}>
                Backlink monitoring helps you detect and investigate changes. It does not guarantee backlink
                retention, recovery, or improvements in search rankings.
              </p>
            </div>
          </section>
        </main>
        <div dangerouslySetInnerHTML={{ __html: MARKETING_FOOTER }} />
      </div>
    </>
  );
}
