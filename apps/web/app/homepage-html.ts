// Marketing homepage (§48). Content/structure from the approved SEO redesign;
// placeholder copy resolved against the shipped product (check cadence is
// "on a schedule you set", claims verified). The six product shots are inline
// `.shot` UI mockups built from the real design language (status colors, the
// distribution bar, the anchor-text distribution), each labelled "Example". The
// LinkPulz charte (dark + purple, #lp-home-scoped CSS), the sticky header and
// the footer live in ./marketing.ts and are shared with the backlink tool pages;
// this file supplies only the homepage's own JSON-LD graph and <main> sections.
import { marketingPage } from "./marketing";

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
      "@id": "https://www.linkpulz.com/#webpage",
      "url": "https://www.linkpulz.com/",
      "name": "Backlink Monitoring Tool: Lost Link Alerts | LinkPulz",
      "description": "Monitor every backlink you earn or buy. Get instant alerts when a link is lost, nofollowed or deindexed.",
      "isPartOf": { "@id": "https://www.linkpulz.com/#website" },
      "about": { "@id": "https://www.linkpulz.com/#software" },
      "primaryImageOfPage": { "@type": "ImageObject", "url": "https://ye8jhaekbbcviws9.public.blob.vercel-storage.com/banniere-dashboard-linkpulz.webp" },
      "breadcrumb": { "@id": "https://www.linkpulz.com/#breadcrumb" },
      "inLanguage": "en"
    },
    {
      "@type": "BreadcrumbList",
      "@id": "https://www.linkpulz.com/#breadcrumb",
      "itemListElement": [
        { "@type": "ListItem", "position": 1, "name": "Home", "item": "https://www.linkpulz.com/" }
      ]
    },
    {
      "@type": "SoftwareApplication",
      "@id": "https://www.linkpulz.com/#software",
      "name": "LinkPulz",
      "applicationCategory": "BusinessApplication",
      "applicationSubCategory": "SEO software",
      "operatingSystem": "Web",
      "description": "Backlink monitoring software that checks your earned and paid backlinks on a schedule and alerts you when a link is lost, redirected, nofollowed, noindexed or deindexed.",
      "url": "https://www.linkpulz.com/",
      "image": "https://ye8jhaekbbcviws9.public.blob.vercel-storage.com/banniere-dashboard-linkpulz.webp",
      "publisher": { "@id": "https://www.linkpulz.com/#organization" },
      "featureList": [
        "Verify your backlinks",
        "Detect lost backlinks",
        "Find broken backlinks",
        "Monitor backlink health",
        "Dofollow, nofollow, sponsored and UGC attribute tracking",
        "Noindex and deindexation detection",
        "Anchor text change detection",
        "Email, Slack and webhook notifications",
        "Client-ready backlink reports"
      ],
      "offers": {
        "@type": "AggregateOffer",
        "priceCurrency": "USD",
        "lowPrice": "29",
        "highPrice": "199",
        "offerCount": "4",
        "description": "Plans from $29/mo with a 10-day free trial, no credit card.",
        "url": "https://www.linkpulz.com/pricing"
      }
    },
    {
      "@type": "FAQPage",
      "@id": "https://www.linkpulz.com/#faq",
      "mainEntity": [
        {
          "@type": "Question",
          "name": "What is backlink monitoring and why do I need it?",
          "acceptedAnswer": { "@type": "Answer", "text": "Backlink monitoring is the automatic, recurring check of the links pointing to your site. Links get removed, redirected, set to nofollow or deindexed all the time, and without monitoring you usually find out long after the damage. Monitoring tells you quickly so you can act." }
        },
        {
          "@type": "Question",
          "name": "How do I know if a backlink has been removed or changed?",
          "acceptedAnswer": { "@type": "Answer", "text": "Add the link to LinkPulz. We re-check the linking page on a schedule and alert you if the link is gone, the page errors or redirects, or the attributes and anchor text change." }
        },
        {
          "@type": "Question",
          "name": "How often does LinkPulz check my backlinks?",
          "acceptedAnswer": { "@type": "Answer", "text": "On a schedule you set: daily, weekly, bi-weekly or monthly, on every plan. Every check is logged in the link's history." }
        },
        {
          "@type": "Question",
          "name": "Can LinkPulz detect when a link changes from dofollow to nofollow?",
          "acceptedAnswer": { "@type": "Answer", "text": "Yes. We track rel attributes (nofollow, sponsored, UGC) on every check and alert you when they change." }
        },
        {
          "@type": "Question",
          "name": "Can I monitor paid or guest-post links?",
          "acceptedAnswer": { "@type": "Answer", "text": "Yes. Tag links as guest post, sponsored or PR, then verify that each one stays live and keeps the attributes you agreed on." }
        },
        {
          "@type": "Question",
          "name": "What's the difference between a backlink checker and a backlink monitoring tool?",
          "acceptedAnswer": { "@type": "Answer", "text": "A checker gives you a one-time snapshot. A monitoring tool checks the same links again and again, keeps the history and alerts you when something changes." }
        },
        {
          "@type": "Question",
          "name": "Do backlinks affect visibility in AI search results?",
          "acceptedAnswer": { "@type": "Answer", "text": "Research has found correlations between strong link profiles and visibility in AI-generated answers, but there is no proven causal link. Backlinks remain valuable for classic search rankings and referral traffic." }
        },
        {
          "@type": "Question",
          "name": "How many links can I monitor on my plan?",
          "acceptedAnswer": { "@type": "Answer", "text": "Monitored-link limits range from 500 on Starter to 15,000 on Agency. You can upgrade at any time as your link portfolio grows." }
        }
      ]
    }
  ]
}`;

const MAIN = `
<!-- ============ HERO ============ -->
<section class="hero" aria-labelledby="h1">
  <div class="wrap grid">
    <div>
      <span class="badge">Backlink monitoring software</span>
      <h1 id="h1">Backlink Monitoring: Protect the Links You Worked Hard to Earn</h1>
      <p class="lead">LinkPulz checks every backlink you've earned or paid for, and tells you the moment one disappears, turns nofollow, redirects or gets deindexed. Stop finding out three months too late.</p>
      <ul class="check">
        <li>Verify your backlinks</li>
        <li>Detect lost backlinks</li>
        <li>Find broken backlinks</li>
        <li>Monitor backlink health</li>
      </ul>
      <div class="cta-row">
        <a class="btn" href="/register">Start free trial</a>
        <a class="btn ghost" href="#how-it-works">See how it works</a>
      </div>
      <p class="micro">No credit card required · Set up in minutes</p>
    </div>
    <img class="hero-img" src="https://ye8jhaekbbcviws9.public.blob.vercel-storage.com/banniere-dashboard-linkpulz.webp" alt="LinkPulz backlink monitoring dashboard showing a lost backlink alert" width="1448" height="1086" fetchpriority="high" style="width:100%;height:auto;border-radius:var(--radius);border:1px solid var(--line)">
  </div>
</section>

<!-- ============ VERIFY BACKLINKS ============ -->
<section id="verify-backlinks" aria-labelledby="verify-h">
  <div class="wrap center">
    <h2 id="verify-h">Verify a Backlink</h2>
    <p class="lead">Check if a backlink exists, whether it's live, and how it's set up. Paste the page that links to you and your URL. Our backlink verification tool does the rest.</p>

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
        <p><b>Want us to re-check it automatically, on a schedule you set?</b></p>
        <a class="btn" href="/register">Monitor this backlink with LinkPulz →</a>
      </div>
    </div>

    <p class="micro">Looking for more? See the full <a href="/verify-backlinks">backlink verification tool</a>.</p>
  </div>
</section>

<!-- ============ LOST BACKLINKS ============ -->
<section class="alt" id="lost-backlinks" aria-labelledby="lost-h">
  <div class="wrap">
    <div class="center">
      <h2 id="lost-h">Lost Backlinks: Find Them Before They Cost You Rankings</h2>
      <p class="lead">You had a backlink, and now it's gone. A lost backlink checker tells you which links disappeared, when, and why, so you can win them back.</p>
    </div>

    <div class="split" style="margin-top:36px">
      <div>
        <h3>Why backlinks disappear</h3>
        <ul class="check">
          <li>The publisher removes the link</li>
          <li>The page is deleted or returns a 404</li>
          <li>The article is edited or rewritten</li>
          <li>A site migration breaks the URL</li>
          <li>A bad redirect sends the link elsewhere</li>
          <li>The link switches to nofollow or the page goes noindex</li>
        </ul>
        <h3 style="margin-top:24px">How to find lost backlinks</h3>
        <p>Add your links to LinkPulz. We re-check each linking page on a schedule and flag any link that is no longer there, with the likely cause.</p>
        <p>Then contact the site owner while the change is recent. <a href="/lost-backlinks">Read our guide on how to regain lost backlinks</a>.</p>
      </div>

      <!-- Example timeline (illustrative, not live data) -->
      <div class="lost-demo" role="img" aria-label="Example: a backlink goes from live yesterday to lost today, with a LinkPulz alert">
        <p class="micro verify-label">Example</p>
        <div class="lost-row">
          <small>Yesterday</small>
          <span class="url">example.com/article</span>
          <span class="status live">● LIVE</span>
        </div>
        <div class="lost-row">
          <small>Today</small>
          <span class="url">example.com/article</span>
          <span class="status lost">● LOST</span>
        </div>
        <div class="lost-alert">
          <b>LinkPulz Alert</b><br>
          Backlink lost 23 minutes ago.
        </div>
      </div>
    </div>

    <div class="center" style="margin-top:36px">
      <a class="btn" href="/register">Get Alerted When a Backlink Disappears</a>
    </div>
  </div>
</section>

<!-- ============ BROKEN BACKLINKS ============ -->
<section id="broken-backlinks" aria-labelledby="broken-h">
  <div class="wrap">
    <div class="center">
      <h2 id="broken-h">Broken Backlinks: Find Them Before They Waste Your Link Equity</h2>
      <p class="lead">A broken backlink isn't always a lost one. The link can still be there while the page it points to, or the page it sits on, is returning an error. A broken backlink checker catches both.</p>
    </div>

    <div class="cards">
      <!-- Case 1 -->
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

      <!-- Case 2 -->
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

    <div class="split" style="margin-top:48px">
      <div>
        <h3>How to find broken backlinks</h3>
        <ul class="check">
          <li>Check the HTTP status of the page that links to you</li>
          <li>Check the HTTP status of the URL it points to</li>
          <li>Spot redirects that send visitors somewhere else</li>
          <li>Fix your own broken pages with a 301 redirect, or ask the site to update the link</li>
        </ul>
      </div>
      <div class="alt-box">
        <p><b>LinkPulz monitors the status of your backlinks and their destinations.</b> It checks both the page your link sits on and the page it points to.</p>
        <p>You get an alert when either side starts returning an error, so you can fix it while it still matters.</p>
        <a class="btn" href="/register">Find Broken Backlinks Automatically</a>
        <p class="micro" style="margin-top:12px">Learn more: <a href="/broken-backlinks">how to find and fix broken backlinks</a>.</p>
      </div>
    </div>
  </div>
</section>

<!-- ============ BACKLINK HEALTH CHECK ============ -->
<section class="alt" id="backlink-health-check" aria-labelledby="health-h">
  <div class="wrap">
    <div class="center">
      <h2 id="health-h">Backlink Health Check: See the State of Your Entire Backlink Profile</h2>
      <p class="lead">It's not just "does this link exist?" Know how healthy your whole backlink portfolio is, and which links need attention first.</p>
    </div>

    <!-- Example dashboard (illustrative data, not live) -->
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

    <div class="cards">
      <div class="card"><h3>One score for your whole profile</h3><p>Get a single health percentage, calculated from the live status of every link you track.</p></div>
      <div class="card"><h3>Every problem, sorted</h3><p>Lost, changed, redirected, nofollow and broken links are grouped so you see what to fix first.</p></div>
      <div class="card"><h3>Track it over time</h3><p>Watch your backlink health improve or drop, and catch trends before they hurt rankings.</p></div>
    </div>

    <div class="center" style="margin-top:36px">
      <a class="btn" href="/register">Check Your Backlink Health</a>
      <p class="micro" style="margin-top:12px">Learn more: <a href="/backlink-health-check">how to run a backlink health check</a>.</p>
    </div>
  </div>
</section>


<!-- ============ WHAT WE MONITOR ============ -->
<section class="alt" id="features" aria-labelledby="monitor-h">
  <div class="wrap">
    <div class="center">
      <h2 id="monitor-h">Backlink monitoring: every way a link can quietly stop working</h2>
      <p class="lead">A link isn't just "there" or "gone". LinkPulz backlink monitoring tracks the changes that cost you rankings and referral traffic.</p>
    </div>
    <div class="cards">
      <div class="card"><div class="ic">✓</div><h3>Verify your backlinks</h3><p>Confirm that each link is still on the page, with the right anchor text and target URL.</p></div>
      <div class="card"><div class="ic">✕</div><h3>Detect lost backlinks</h3><p>Get alerted when the page removes your link or disappears altogether.</p></div>
      <div class="card"><div class="ic">404</div><h3>Find broken backlinks</h3><p>Spot linking pages that return a 404 or 5xx error, and links that now redirect elsewhere.</p></div>
      <div class="card"><div class="ic">♥</div><h3>Monitor backlink health</h3><p>Track status, attributes and indexation for every link in one view.</p></div>
      <div class="card"><div class="ic">rel</div><h3>Attribute changes</h3><p>Dofollow switches to nofollow, sponsored or UGC.</p></div>
      <div class="card"><div class="ic">NI</div><h3>Noindex</h3><p>The linking page is set to noindex and passes little value.</p></div>
      <div class="card"><div class="ic">G</div><h3>Deindexation</h3><p>The linking page drops out of Google's index.</p></div>
      <div class="card"><div class="ic">Aa</div><h3>Anchor text changes</h3><p>Your agreed anchor is edited or replaced.</p></div>
    </div>
  </div>
</section>

<!-- ============ HOW IT WORKS ============ -->
<section id="how-it-works" aria-labelledby="how-h">
  <div class="wrap">
    <div class="center">
      <h2 id="how-h">Start backlink monitoring in three steps</h2>
    </div>
    <div class="steps">
      <div class="step">
        <div class="shot" role="img" aria-label="Importing backlinks into LinkPulz">
          <div class="shot-bar"><span class="shot-dots"><i></i><i></i><i></i></span><span class="shot-ttl">Import backlinks</span></div>
          <div class="shot-body">
            <span class="ex">Example</span>
            <div class="srow"><span aria-hidden="true">📄</span><span class="surl">backlinks-q3.xlsx</span><span class="sbadge live">48 rows</span></div>
            <div class="stiles"><div class="stile"><b>46</b><small>Imported</small></div><div class="stile"><b>2</b><small>Duplicates</small></div><div class="stile"><b>0</b><small>Errors</small></div></div>
            <div class="stags"><span class="stag">CSV · Excel · GSC</span><span class="stag">Tag: Q3 outreach</span><span class="stag">Client: Acme</span></div>
          </div>
        </div>
        <h3>Add your links</h3>
        <p>Paste them, upload a CSV, or import your referring domains from Google Search Console. Tag each one by campaign, client or cost.</p>
      </div>
      <div class="step">
        <div class="shot" role="img" aria-label="Automatic backlink checks on a schedule">
          <div class="shot-bar"><span class="shot-dots"><i></i><i></i><i></i></span><span class="shot-ttl">Backlinks · Acme</span></div>
          <div class="shot-body">
            <span class="ex">Example data</span>
            <div class="sbar" role="img" aria-label="82% live, 8% changed, 6% lost, 4% redirected"><i style="width:82%;background:var(--accent)"></i><i style="width:8%;background:var(--warn)"></i><i style="width:6%;background:var(--danger)"></i><i style="width:4%;background:#1c7ed6"></i></div>
            <div class="srow"><span class="surl">forbes.com/…/best-tools</span><span class="sbadge live">Live</span></div>
            <div class="srow"><span class="surl">blog.example.com/seo</span><span class="sbadge changed">Changed</span></div>
            <div class="srow"><span class="surl">news.site/review</span><span class="sbadge lost">Lost</span></div>
            <div class="stags"><span class="stag">⏱ Next check in 2 days</span><span class="stag">Weekly schedule</span></div>
          </div>
        </div>
        <h3>We check them automatically</h3>
        <p>LinkPulz backlink monitoring re-crawls each linking page on a schedule and compares it to the last known state.</p>
      </div>
      <div class="step">
        <div class="shot" role="img" aria-label="A backlink change alert">
          <div class="shot-bar"><span class="shot-dots"><i></i><i></i><i></i></span><span class="shot-ttl">Alert</span></div>
          <div class="shot-body">
            <span class="ex">Example</span>
            <div class="alert-mock" style="margin-top:0"><b>Link lost</b> · example.com/best-seo-tools<br><span class="sdiff"><del>dofollow · anchor “backlink monitoring”</del> → <ins>link removed</ins></span><br><span class="smuted">Detected today</span></div>
            <div class="stags"><span class="stag">✉ Email</span><span class="stag">Slack</span><span class="stag">Webhook</span></div>
          </div>
        </div>
        <h3>You get alerted</h3>
        <p>Email, Slack or webhook notifications the moment something changes, with the exact change highlighted.</p>
      </div>
    </div>
  </div>
</section>

<!-- ============ ALERTS & REPORTING ============ -->
<section class="alt" aria-labelledby="alerts-h">
  <div class="wrap split">
    <div>
      <h2 id="alerts-h">Alerts that tell you what changed, not just that something did</h2>
      <p>Backlink monitoring is only useful when the alert is specific. Every alert includes the source URL, your target URL, what changed (before and after), and the date it was detected. Route alerts to the right person by client or campaign, or get a weekly digest instead of real-time pings.</p>
      <ul class="check">
        <li>Email, Slack and webhook alerts</li>
        <li>CSV export of every link and its full change history</li>
        <li>Full change history for every link</li>
      </ul>
      <div class="alert-mock" aria-hidden="true">
        <b>Link lost</b> · example.com/best-seo-tools<br>
        Before: <i>dofollow, anchor "backlink monitoring"</i> → After: <i>link removed</i><br>
        Detected: today
      </div>
    </div>
    <div class="shot" role="img" aria-label="Alert email and full change history">
      <div class="shot-bar"><span class="shot-dots"><i></i><i></i><i></i></span><span class="shot-ttl">Alert email</span></div>
      <div class="shot-body">
        <span class="ex">Example</span>
        <div class="smail">
          <div class="smail-h"><b>A backlink was lost</b><br><span class="smuted" style="font-size:.7rem">LinkPulz → you@acme.com</span></div>
          <div class="smail-b">
            <div class="sdiff"><b>Source</b> example.com/best-seo-tools<br><b>Target</b> acme.com/pricing<br><b>Change</b> <del>dofollow · anchor “backlink monitoring”</del> → <ins>link removed</ins><br><b>Detected</b> today</div>
            <div style="margin-top:11px"><span class="sbtn">Open in LinkPulz</span></div>
          </div>
        </div>
        <div>
          <div class="cap">Full change history</div>
          <ul class="stl">
            <li><b>Lost</b> <span class="smuted">· today</span></li>
            <li>Anchor changed <span class="smuted">· 2 weeks ago</span></li>
            <li>Verified live <span class="smuted">· Jun 3</span></li>
            <li><b>Added</b> <span class="smuted">· May 20</span></li>
          </ul>
          <span class="sbtn ghost">⤓ Export CSV</span>
        </div>
      </div>
    </div>
  </div>
</section>

<!-- ============ USE CASES ============ -->
<section aria-labelledby="use-h">
  <div class="wrap">
    <div class="center"><h2 id="use-h">Built for people who build links for a living</h2></div>
    <div class="cards">
      <div class="card"><h3>Agencies</h3><p>Prove the links you delivered are still live, and send client-ready reports.</p></div>
      <div class="card"><h3>Digital PR teams</h3><p>Track coverage links after the campaign ends and catch silent removals.</p></div>
      <div class="card"><h3>In-house SEOs</h3><p>Protect the links behind your most important pages.</p></div>
      <div class="card"><h3>Guest post & sponsored buyers</h3><p>Verify you're getting what you paid for, for as long as you paid for it.</p></div>
    </div>
  </div>
</section>

<!-- ============ RECOVERY ============ -->
<section class="alt" aria-labelledby="rec-h">
  <div class="wrap split rev">
    <div class="shot" role="img" aria-label="Lost link details to help you win it back">
      <div class="shot-bar"><span class="shot-dots"><i></i><i></i><i></i></span><span class="shot-ttl">Lost link · details</span></div>
      <div class="shot-body">
        <span class="ex">Example</span>
        <div class="srow"><span class="surl">example.com/best-seo-tools</span><span class="sbadge lost">Lost</span></div>
        <div class="stiles"><div class="stile"><b>3 days</b><small>Since lost</small></div><div class="stile"><b>$120</b><small>Paid placement</small></div><div class="stile"><b>example.com</b><small>Referring site</small></div></div>
        <div class="sdiff"><b>What changed</b> <del>dofollow · anchor “backlink monitoring”</del> → <ins>link removed</ins></div>
        <div class="smail"><div class="smail-h"><b>Reach out</b> <span class="smuted" style="font-size:.7rem">· editor@example.com</span></div><div class="smail-b smuted">Hi — I noticed the link to acme.com in your “best SEO tools” post was removed in a recent edit. Would you be open to restoring it?</div></div>
        <div class="stags"><span class="sbtn">Draft outreach</span><span class="stag">Owner on file</span></div>
      </div>
    </div>
    <div>
      <h2 id="rec-h">Found a lost link? Win it back.</h2>
      <p>When LinkPulz flags a lost or changed link, you see who owns the page, when the link disappeared and what changed, so you can contact the site while the request is still fresh.</p>
      <p>See our guide to <a href="/lost-backlinks">recovering lost backlinks</a>.</p>
    </div>
  </div>
</section>

<!-- ============ INSIGHTS ============ -->
<section aria-labelledby="ins-h">
  <div class="wrap split">
    <div>
      <h2 id="ins-h">The data you need, nothing you don't</h2>
      <p>See referring domains, anchor text distribution, link attributes and link age in one view. We show you what's measurable and don't hand you an opaque "toxic score" to guess at, we'd rather point you to <a href="https://developers.google.com/search/docs/essentials/spam-policies" rel="nofollow noopener" target="_blank">Google's own guidance on link spam</a> than make one up.</p>
    </div>
    <div class="shot" role="img" aria-label="Backlink profile with anchor text distribution">
      <div class="shot-bar"><span class="shot-dots"><i></i><i></i><i></i></span><span class="shot-ttl">Backlink profile · Acme</span></div>
      <div class="shot-body">
        <span class="ex">Example data</span>
        <div>
          <div class="cap">Anchor text distribution</div>
          <div class="sbar" role="img" aria-label="38% branded, 24% keyword, 18% naked URL, 12% generic, 8% empty"><i style="width:38%;background:var(--brand)"></i><i style="width:24%;background:var(--violet)"></i><i style="width:18%;background:var(--accent)"></i><i style="width:12%;background:var(--muted)"></i><i style="width:8%;background:var(--ph-line)"></i></div>
          <div class="sleg" style="margin-top:9px">
            <span><i class="sdot" style="background:var(--brand)"></i>Branded <b>38%</b></span>
            <span><i class="sdot" style="background:var(--violet)"></i>Keyword <b>24%</b></span>
            <span><i class="sdot" style="background:var(--accent)"></i>Naked URL <b>18%</b></span>
            <span><i class="sdot" style="background:var(--muted)"></i>Generic <b>12%</b></span>
            <span><i class="sdot" style="background:var(--ph-line)"></i>Empty <b>8%</b></span>
          </div>
        </div>
        <div>
          <div class="srow smuted" style="font-size:.64rem;text-transform:uppercase;letter-spacing:.04em"><span class="surl">Referring page</span><span>Anchor</span><span>Age</span></div>
          <div class="srow"><span class="surl">forbes.com/…/tools</span><span>LinkPulz</span><span class="smuted">8mo</span></div>
          <div class="srow"><span class="surl">blog.example.com/seo</span><span>best backlink tool</span><span class="smuted">3mo</span></div>
          <div class="srow"><span class="surl">news.site/review</span><span>acme.com</span><span class="smuted">6w</span></div>
        </div>
        <div class="stags"><span class="stag">42 referring domains</span><span class="stag">No toxic score — just the data</span></div>
      </div>
    </div>
  </div>
</section>

<!-- ============ COMPARISON ============ -->
<section class="alt" id="compare" aria-labelledby="cmp-h">
  <div class="wrap">
    <div class="center">
      <h2 id="cmp-h">LinkPulz vs. other backlink monitoring tools</h2>
      <p class="lead">Some tools monitor links, some verify them once, and some bundle monitoring inside a full SEO suite. Here is how they compare.</p>
    </div>

    <div class="tablewrap">
      <table class="compare">
        <caption class="sr-only">Comparison of LinkPulz with other backlink tools</caption>
        <thead>
          <tr>
            <th scope="col"></th>
            <th scope="col" class="us">LinkPulz</th>
            <th scope="col">Linkody</th>
            <th scope="col">Monitor Backlinks</th>
            <th scope="col">VerifyBacklinks</th>
            <th scope="col">All-in-one SEO suites</th>
          </tr>
        </thead>
        <tbody>
          <tr>
            <th scope="row">Focus</th>
            <td class="us">Backlink monitoring</td>
            <td>Backlink monitoring for SEOs and agencies</td>
            <td>Backlink monitoring, competitor tracking and keyword rank tracking</td>
            <td>Audit of an uploaded list of backlinks</td>
            <td>Hundreds of SEO features, with a backlink module</td>
          </tr>
          <tr>
            <th scope="row">How it checks</th>
            <td class="us">On a schedule you set (daily to monthly)</td>
            <td>Continuous, with ongoing monitoring</td>
            <td>Daily monitoring</td>
            <td>One-off audit, paid per audit</td>
            <td>Continuous, inside the backlink module</td>
          </tr>
          <tr>
            <th scope="row">Lost link alerts</th>
            <td class="us">Yes — email, Slack or webhook</td>
            <td>Email alerts for new and lost links</td>
            <td>Alerts for removed or broken backlinks</td>
            <td>No alerts, report only</td>
            <td>Yes, in tools such as SE Ranking and Ahrefs</td>
          </tr>
          <tr>
            <th scope="row">Broken link detection</th>
            <td class="us">Yes — source and destination</td>
            <td>Not stated on its site</td>
            <td>Yes, for broken backlinks</td>
            <td>Yes, flags HTTP errors during the audit</td>
            <td>Yes, in tools such as Ahrefs</td>
          </tr>
          <tr>
            <th scope="row">Reports for clients</th>
            <td class="us">CSV export</td>
            <td>White-label PDF reports</td>
            <td>White-label plan available</td>
            <td>Excel and PDF export</td>
            <td>Varies by suite</td>
          </tr>
          <tr>
            <th scope="row">Pricing model</th>
            <td class="us">Subscription, from $29/mo, 10-day free trial</td>
            <td>Subscription, from $14.90/mo, with a 30-day free trial</td>
            <td>Subscription, from $29/mo</td>
            <td>Pay per audit, from €29 for up to 500 links</td>
            <td>Subscription, roughly $55 to $165+/mo to start</td>
          </tr>
        </tbody>
      </table>
    </div>

    <p class="micro center" style="margin-top:16px">Competitor information comes from their public websites and independent comparison articles, last checked October 2026. Features and prices change, so check each vendor for current details.</p>

    <div class="center" style="margin-top:24px">
      <a class="btn" href="/register">Try LinkPulz free</a>
    </div>
  </div>
</section>

<!-- ============ PRICING ============ -->
<section id="pricing" aria-labelledby="price-h">
  <div class="wrap">
    <div class="center">
      <h2 id="price-h">Simple pricing, based on links monitored</h2>
      <p class="lead">Every plan includes a 10-day free trial, no credit card, and checks every backlink on the schedule you set (daily to monthly).</p>
    </div>
    <div class="plans">
      <div class="plan"><h3>Starter</h3><div class="price-block price-monthly"><div class="price">$29<small>/mo</small></div><div class="per">billed monthly</div></div><div class="limits"><span><b>500</b> backlinks</span><span><b>3</b> projects</span><span><b>1</b> user</span></div><a class="btn ghost" href="/register">Start free trial</a></div>
      <div class="plan hl"><span class="pop">Most popular</span><h3>Pro</h3><div class="price-block price-monthly"><div class="price">$59<small>/mo</small></div><div class="per">billed monthly</div></div><div class="limits"><span><b>2,500</b> backlinks</span><span><b>10</b> projects</span><span><b>3</b> users</span></div><a class="btn" href="/register">Start free trial</a></div>
      <div class="plan"><h3>Business</h3><div class="price-block price-monthly"><div class="price">$99<small>/mo</small></div><div class="per">billed monthly</div></div><div class="limits"><span><b>5,000</b> backlinks</span><span><b>25</b> projects</span><span><b>5</b> users</span></div><a class="btn ghost" href="/register">Start free trial</a></div>
      <div class="plan"><h3>Agency</h3><div class="price-block price-monthly"><div class="price">$199<small>/mo</small></div><div class="per">billed monthly</div></div><div class="limits"><span><b>15,000</b> backlinks</span><span><b>75</b> projects</span><span><b>15</b> users</span></div><a class="btn ghost" href="/register">Start free trial</a></div>
    </div>
    <div class="center" style="margin-top:28px"><a class="btn ghost" href="/pricing">See full pricing &amp; annual plans →</a></div>
  </div>
</section>


<!-- ============ FAQ (must match FAQPage JSON-LD) ============ -->
<section id="faq" aria-labelledby="faq-h">
  <div class="wrap">
    <div class="center"><h2 id="faq-h">Backlink monitoring: frequently asked questions</h2></div>
    <div class="faq">
      <details><summary>What is backlink monitoring and why do I need it?</summary><p>Backlink monitoring is the automatic, recurring check of the links pointing to your site. Links get removed, redirected, set to nofollow or deindexed all the time, and without monitoring you usually find out long after the damage. Monitoring tells you quickly so you can act.</p></details>
      <details><summary>How do I know if a backlink has been removed or changed?</summary><p>Add the link to LinkPulz. We re-check the linking page on a schedule and alert you if the link is gone, the page errors or redirects, or the attributes and anchor text change.</p></details>
      <details><summary>How often does LinkPulz check my backlinks?</summary><p>On a schedule you set: daily, weekly, bi-weekly or monthly, on every plan. Every check is logged in the link's history.</p></details>
      <details><summary>Can LinkPulz detect when a link changes from dofollow to nofollow?</summary><p>Yes. We track rel attributes (nofollow, sponsored, UGC) on every check and alert you when they change.</p></details>
      <details><summary>Can I monitor paid or guest-post links?</summary><p>Yes. Tag links as guest post, sponsored or PR, then verify that each one stays live and keeps the attributes you agreed on.</p></details>
      <details><summary>What's the difference between a backlink checker and a backlink monitoring tool?</summary><p>A checker gives you a one-time snapshot. A monitoring tool checks the same links again and again, keeps the history and alerts you when something changes.</p></details>
      <details><summary>Do backlinks affect visibility in AI search results?</summary><p>Research has found correlations between strong link profiles and visibility in AI-generated answers, but there is no proven causal link. Backlinks remain valuable for classic search rankings and referral traffic.</p></details>
      <details><summary>How many links can I monitor on my plan?</summary><p>Monitored-link limits range from 500 on Starter to 15,000 on Agency. You can upgrade at any time as your link portfolio grows.</p></details>
    </div>
  </div>
</section>

<!-- ============ FINAL CTA ============ -->
<section class="final" aria-labelledby="final-h">
  <div class="wrap">
    <h2 id="final-h">Don't let your best links disappear unnoticed</h2>
    <p class="lead">Every lost backlink is lost authority and lost traffic. Start monitoring your links today and get alerted the moment something changes.</p>
    <div class="cta-row" style="justify-content:center"><a class="btn" href="/register">Start free trial</a></div>
  </div>
</section>
`;

export const HOMEPAGE_HTML = marketingPage(JSONLD, MAIN);
