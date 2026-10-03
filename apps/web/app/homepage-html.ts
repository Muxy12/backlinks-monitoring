// Marketing homepage (§48). Content/structure from the approved SEO redesign,
// kept verbatim (sections, copy, FAQ, JSON-LD, TO CONFIRM placeholders). The
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
        "url": "https://www.linkpulz.com/images/logo.png"
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
      "primaryImageOfPage": { "@type": "ImageObject", "url": "https://www.linkpulz.com/images/og-linkpulz.jpg" },
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
      "image": "https://www.linkpulz.com/images/og-linkpulz.jpg",
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
        "@type": "Offer",
        "price": "0",
        "priceCurrency": "USD",
        "description": "Free trial. TO CONFIRM: replace with your real starting plan, price and currency.",
        "url": "https://www.linkpulz.com/#pricing"
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
          "acceptedAnswer": { "@type": "Answer", "text": "[TO CONFIRM: e.g. every 24 hours on all plans]. Every check is logged in the link's history." }
        },
        {
          "@type": "Question",
          "name": "Can LinkPulz detect when a link changes from dofollow to nofollow?",
          "acceptedAnswer": { "@type": "Answer", "text": "Yes. We track rel attributes (nofollow, sponsored, UGC) on every check and alert you when they change. [TO CONFIRM]" }
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
          "acceptedAnswer": { "@type": "Answer", "text": "[TO CONFIRM per plan]. You can upgrade at any time as your link portfolio grows." }
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
      <p class="micro">No credit card required <span class="tc">TO CONFIRM</span> · Set up in under 5 minutes <span class="tc">TO CONFIRM</span></p>
    </div>
    <!-- PHOTO PLACEHOLDER: product screenshot -->
    <div class="ph hero-img" role="img" aria-label="LinkPulz dashboard showing a Link lost alert">
      <div>IMAGE PLACEHOLDER: dashboard screenshot<small>1200×900 · WebP · show a "Link lost" alert with source URL, target URL and date. Replace with &lt;img src="…" alt="LinkPulz backlink monitoring dashboard with a Link lost alert" width="1200" height="900" fetchpriority="high"&gt;</small></div>
    </div>
  </div>
</section>

<!-- ============ FREE MINI-TOOL ============ -->
<section class="alt" id="checker" aria-labelledby="tool-h">
  <div class="wrap center">
    <h2 id="tool-h">Is your backlink still live?</h2>
    <p class="lead">Paste the page that links to you and your target URL. We'll check HTTP status, link attributes and indexation in seconds.</p>
    <div class="tool">
      <form action="/tools/backlink-checker" method="get">
        <input type="url" name="source" placeholder="Source URL (page that links to you)" required aria-label="Source URL">
        <input type="url" name="target" placeholder="Target URL (your page)" required aria-label="Target URL">
        <button class="btn" type="submit">Check my link</button>
      </form>
      <div class="result-list" aria-label="What the checker reports">
        <span>Link found / not found</span><span>HTTP status</span><span>dofollow · nofollow · sponsored · UGC</span><span>Indexed or not</span><span>Anchor text</span>
      </div>
      <p class="micro" style="margin:16px 0 0">Want this checked automatically? <a href="/register">Monitor this link free</a> <span class="tc">frequency: TO CONFIRM</span></p>
    </div>
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
        <p><b>Want us to check it automatically every day?</b> <span class="tc">frequency: TO CONFIRM</span></p>
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
        <p>Add your links to LinkPulz. We re-check each linking page on a schedule and flag any link that is no longer there, with the likely cause <span class="tc">TO CONFIRM</span>.</p>
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
        <p><b>LinkPulz monitors the status of your backlinks and their destinations.</b> <span class="tc">TO CONFIRM that both are checked</span></p>
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
      <div class="card"><h3>One score for your whole profile</h3><p>Get a single health percentage, calculated from the status of every link you track <span class="tc">TO CONFIRM how it's calculated</span>.</p></div>
      <div class="card"><h3>Every problem, sorted</h3><p>Lost, changed, redirected, nofollow and broken links are grouped so you see what to fix first.</p></div>
      <div class="card"><h3>Track it over time</h3><p>Watch your backlink health improve or drop, and catch trends before they hurt rankings <span class="tc">TO CONFIRM history</span>.</p></div>
    </div>

    <div class="center" style="margin-top:36px">
      <a class="btn" href="/register">Check Your Backlink Health</a>
      <p class="micro" style="margin-top:12px">Learn more: <a href="/backlink-health-check">how to run a backlink health check</a>.</p>
    </div>
  </div>
</section>

<!-- ============ QUICK PROOF ============ -->
<section aria-labelledby="proof-h">
  <div class="wrap center">
    <h2 id="proof-h">Trusted by SEO teams that live and die by their links</h2>
    <div class="stats">
      <div class="stat"><b><span class="tc">X</span></b>links monitored</div>
      <div class="stat"><b><span class="tc">24h</span></b>check frequency</div>
      <div class="stat"><b><span class="tc">X min</span></b>average alert delay</div>
    </div>
    <p class="micro"><span class="tc">TO CONFIRM</span> Replace with verifiable numbers or remove this block.</p>
    <div class="logos" aria-label="Customer logos">
      <div class="ph logo-ph">LOGO 1<small>only with permission</small></div>
      <div class="ph logo-ph">LOGO 2</div>
      <div class="ph logo-ph">LOGO 3</div>
      <div class="ph logo-ph">LOGO 4</div>
      <div class="ph logo-ph">LOGO 5</div>
    </div>
  </div>
</section>

<!-- ============ WHAT WE MONITOR ============ -->
<section class="alt" id="features" aria-labelledby="monitor-h">
  <div class="wrap">
    <div class="center">
      <h2 id="monitor-h">Every way a backlink can quietly stop working</h2>
      <p class="lead">A link isn't just "there" or "gone". LinkPulz tracks the changes that cost you rankings and referral traffic.</p>
    </div>
    <div class="cards">
      <div class="card"><div class="ic">✓</div><h3>Verify your backlinks</h3><p>Confirm that each link is still on the page, with the right anchor text and target URL.</p></div>
      <div class="card"><div class="ic">✕</div><h3>Detect lost backlinks</h3><p>Get alerted when the page removes your link or disappears altogether.</p></div>
      <div class="card"><div class="ic">404</div><h3>Find broken backlinks</h3><p>Spot linking pages that return a 404 or 5xx error, and links that now redirect elsewhere.</p></div>
      <div class="card"><div class="ic">♥</div><h3>Monitor backlink health</h3><p>Track status, attributes and indexation for every link in one view <span class="tc">TO CONFIRM</span>.</p></div>
      <div class="card"><div class="ic">rel</div><h3>Attribute changes</h3><p>Dofollow switches to nofollow, sponsored or UGC.</p></div>
      <div class="card"><div class="ic">NI</div><h3>Noindex</h3><p>The linking page is set to noindex and passes little value.</p></div>
      <div class="card"><div class="ic">G</div><h3>Deindexation</h3><p>The linking page drops out of Google's index <span class="tc">TO CONFIRM how detected</span>.</p></div>
      <div class="card"><div class="ic">Aa</div><h3>Anchor text changes</h3><p>Your agreed anchor is edited or replaced.</p></div>
    </div>
  </div>
</section>

<!-- ============ HOW IT WORKS ============ -->
<section id="how-it-works" aria-labelledby="how-h">
  <div class="wrap">
    <div class="center">
      <h2 id="how-h">Start monitoring backlinks in three steps</h2>
    </div>
    <div class="steps">
      <div class="step">
        <div class="ph" role="img" aria-label="Importing backlinks">IMAGE PLACEHOLDER<small>Step 1 screenshot · 640×400</small></div>
        <h3>Add your links</h3>
        <p>Paste them, upload a CSV, or import from Google Search Console <span class="tc">TO CONFIRM</span>. Tag each one by campaign, client or cost.</p>
      </div>
      <div class="step">
        <div class="ph" role="img" aria-label="Automatic checks">IMAGE PLACEHOLDER<small>Step 2 screenshot · 640×400</small></div>
        <h3>We check them automatically</h3>
        <p>LinkPulz re-crawls each linking page on a schedule and compares it to the last known state.</p>
      </div>
      <div class="step">
        <div class="ph" role="img" aria-label="Alert notification">IMAGE PLACEHOLDER<small>Step 3 screenshot · 640×400</small></div>
        <h3>You get alerted</h3>
        <p>Email, Slack or webhook notifications the moment something changes <span class="tc">TO CONFIRM channels</span>, with the exact change highlighted.</p>
      </div>
    </div>
  </div>
</section>

<!-- ============ ALERTS & REPORTING ============ -->
<section class="alt" aria-labelledby="alerts-h">
  <div class="wrap split">
    <div>
      <h2 id="alerts-h">Alerts that tell you what changed, not just that something did</h2>
      <p>Every alert includes the source URL, your target URL, what changed (before and after), and the date it was detected. Route alerts to the right person by client or campaign, or get a weekly digest instead of real-time pings.</p>
      <ul class="check">
        <li>Email, Slack and webhook alerts <span class="tc">TO CONFIRM</span></li>
        <li>Scheduled PDF or shareable reports for clients <span class="tc">TO CONFIRM</span></li>
        <li>Full change history for every link</li>
      </ul>
      <div class="alert-mock" aria-hidden="true">
        <b>Link lost</b> · example.com/best-seo-tools<br>
        Before: <i>dofollow, anchor "backlink monitoring"</i> → After: <i>link removed</i><br>
        Detected: today
      </div>
    </div>
    <div class="ph" role="img" aria-label="Alert and report preview">IMAGE PLACEHOLDER<small>Alert email / Slack message / report preview · 900×700</small></div>
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
    <div class="ph" role="img" aria-label="Lost link recovery workflow">IMAGE PLACEHOLDER<small>Recovery workflow screenshot · 900×700</small></div>
    <div>
      <h2 id="rec-h">Found a lost link? Win it back.</h2>
      <p>When LinkPulz flags a lost or changed link, you see who owns the page, when the link disappeared and what changed, so you can contact the site while the request is still fresh.</p>
      <p>Track each recovery attempt until the link is back <span class="tc">TO CONFIRM workflow feature</span>. See our guide to <a href="/lost-backlinks">recovering lost backlinks</a>.</p>
    </div>
  </div>
</section>

<!-- ============ INSIGHTS ============ -->
<section aria-labelledby="ins-h">
  <div class="wrap split">
    <div>
      <h2 id="ins-h">The data you need, nothing you don't</h2>
      <p>See referring domains, anchor text distribution, link attributes and link age in one view. We show you what's measurable and don't hand you an opaque "toxic score" to guess at.</p>
    </div>
    <div class="ph" role="img" aria-label="Backlink profile view">IMAGE PLACEHOLDER<small>Backlink profile table / chart · 900×700</small></div>
  </div>
</section>

<!-- ============ COMPARISON ============ -->
<section class="alt" aria-labelledby="cmp-h">
  <div class="wrap">
    <div class="center">
      <h2 id="cmp-h">LinkPulz vs. all-in-one SEO suites</h2>
      <p class="lead">Only publish facts you can verify today.</p>
    </div>
    <div class="tablewrap">
      <table>
        <thead><tr><th></th><th>LinkPulz</th><th>Typical all-in-one SEO suite</th></tr></thead>
        <tbody>
          <tr><th>Focus</th><td>Backlink monitoring only</td><td>Hundreds of SEO features</td></tr>
          <tr><th>Alerts on lost/changed links</th><td><span class="tc">TO CONFIRM</span></td><td><span class="tc">verify per competitor</span></td></tr>
          <tr><th>Check frequency</th><td><span class="tc">TO CONFIRM</span></td><td><span class="tc">verify per competitor</span></td></tr>
          <tr><th>Starting price</th><td><span class="tc">TO CONFIRM</span></td><td><span class="tc">verify per competitor</span></td></tr>
        </tbody>
      </table>
    </div>
  </div>
</section>

<!-- ============ PRICING ============ -->
<section id="pricing" aria-labelledby="price-h">
  <div class="wrap">
    <div class="center">
      <h2 id="price-h">Simple pricing, based on links monitored</h2>
      <p class="lead"><span class="tc">TO CONFIRM</span> plans, limits and prices. Keep the plans visible on the homepage.</p>
    </div>
    <div class="plans">
      <div class="plan"><h3>Starter</h3><div class="price"><span class="tc">$X</span><small class="micro">/mo</small></div><ul><li><span class="tc">X</span> links monitored</li><li>Email alerts</li></ul><a class="btn ghost" href="/register">Start free trial</a></div>
      <div class="plan hl"><h3>Pro</h3><div class="price"><span class="tc">$X</span><small class="micro">/mo</small></div><ul><li><span class="tc">X</span> links monitored</li><li>Slack &amp; webhook alerts</li><li>Client reports</li></ul><a class="btn" href="/register">Start free trial</a></div>
      <div class="plan"><h3>Agency</h3><div class="price"><span class="tc">$X</span><small class="micro">/mo</small></div><ul><li><span class="tc">X</span> links monitored</li><li>Team seats</li><li>White-label reports</li></ul><a class="btn ghost" href="/register">Start free trial</a></div>
    </div>
  </div>
</section>

<!-- ============ REVIEWS (real ones only) ============ -->
<section class="alt" aria-labelledby="rev-h">
  <div class="wrap">
    <div class="center"><h2 id="rev-h">What users say</h2>
      <p class="lead">Add real testimonials only (name, role, company). If you don't have any yet, remove this section and do not add review markup.</p></div>
    <div class="reviews">
      <div class="review"><p>“<span class="tc">Real customer quote</span>”</p><div class="who"><div class="ph avatar">PHOTO</div><div><b>Name</b><br><span class="micro">Role, Company</span></div></div></div>
      <div class="review"><p>“<span class="tc">Real customer quote</span>”</p><div class="who"><div class="ph avatar">PHOTO</div><div><b>Name</b><br><span class="micro">Role, Company</span></div></div></div>
      <div class="review"><p>“<span class="tc">Real customer quote</span>”</p><div class="who"><div class="ph avatar">PHOTO</div><div><b>Name</b><br><span class="micro">Role, Company</span></div></div></div>
    </div>
  </div>
</section>

<!-- ============ FAQ (must match FAQPage JSON-LD) ============ -->
<section id="faq" aria-labelledby="faq-h">
  <div class="wrap">
    <div class="center"><h2 id="faq-h">Backlink monitoring: frequently asked questions</h2></div>
    <div class="faq">
      <details><summary>What is backlink monitoring and why do I need it?</summary><p>Backlink monitoring is the automatic, recurring check of the links pointing to your site. Links get removed, redirected, set to nofollow or deindexed all the time, and without monitoring you usually find out long after the damage. Monitoring tells you quickly so you can act.</p></details>
      <details><summary>How do I know if a backlink has been removed or changed?</summary><p>Add the link to LinkPulz. We re-check the linking page on a schedule and alert you if the link is gone, the page errors or redirects, or the attributes and anchor text change.</p></details>
      <details><summary>How often does LinkPulz check my backlinks?</summary><p>[TO CONFIRM: e.g. every 24 hours on all plans]. Every check is logged in the link's history.</p></details>
      <details><summary>Can LinkPulz detect when a link changes from dofollow to nofollow?</summary><p>Yes. We track rel attributes (nofollow, sponsored, UGC) on every check and alert you when they change. [TO CONFIRM]</p></details>
      <details><summary>Can I monitor paid or guest-post links?</summary><p>Yes. Tag links as guest post, sponsored or PR, then verify that each one stays live and keeps the attributes you agreed on.</p></details>
      <details><summary>What's the difference between a backlink checker and a backlink monitoring tool?</summary><p>A checker gives you a one-time snapshot. A monitoring tool checks the same links again and again, keeps the history and alerts you when something changes.</p></details>
      <details><summary>Do backlinks affect visibility in AI search results?</summary><p>Research has found correlations between strong link profiles and visibility in AI-generated answers, but there is no proven causal link. Backlinks remain valuable for classic search rankings and referral traffic.</p></details>
      <details><summary>How many links can I monitor on my plan?</summary><p>[TO CONFIRM per plan]. You can upgrade at any time as your link portfolio grows.</p></details>
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
