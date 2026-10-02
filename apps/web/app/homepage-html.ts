// Marketing homepage (§48). Content/structure from the approved SEO redesign,
// kept verbatim (sections, copy, FAQ, JSON-LD, TO CONFIRM placeholders). Only the
// visual charte is adapted to match the rest of LinkPulz: the design's own CSS
// variables are remapped from blue/light to the app's dark + purple palette
// (globals.css tokens), and the light/dark switch is wired to the app's
// `data-theme` toggle instead of the OS `prefers-color-scheme`. Every rule is
// scoped under #lp-home so it can't collide with the app's global stylesheets
// (globals.css / theme-light.css). Static markup, Server Component (zero client JS).
export const HOMEPAGE_HTML = `
<style>
#lp-home{
  --bg:#07090d; --bg-alt:#0d1117; --ink:#f7f8fa; --muted:#8b949e; --line:rgba(255,255,255,.08);
  --brand:#8b5cf6; --brand-dark:#6d28d9; --accent:#36d399; --danger:#fb7185; --warn:#fbbf24;
  --ph:#111720; --ph-line:rgba(255,255,255,.15); --radius:14px; --max:1120px;
  font:17px/1.6 Inter,ui-sans-serif,system-ui,-apple-system,BlinkMacSystemFont,"Segoe UI",sans-serif;
  color:var(--ink);
  background:radial-gradient(circle at 50% -10%,rgba(139,92,246,.16),transparent 34%),var(--bg);
  min-height:100vh;
}
:root[data-theme="light"] #lp-home{
  --bg:#ffffff; --bg-alt:#f4f7fb; --ink:#0f1b2d; --muted:#51607a; --line:#dbe3ee;
  --brand:#7c3aed; --brand-dark:#6d28d9; --accent:#12b886; --danger:#e03131; --warn:#f08c00;
  --ph:#e8eef7; --ph-line:#b7c4d9;
}
#lp-home,#lp-home *{box-sizing:border-box}
#lp-home img{max-width:100%;display:block}
#lp-home a{color:var(--brand)}
#lp-home .wrap{max-width:var(--max);margin:0 auto;padding:0 20px}
#lp-home section{padding:72px 0}
#lp-home section.alt{background:var(--bg-alt)}
#lp-home h1,#lp-home h2,#lp-home h3{line-height:1.2;margin:0 0 .5em}
#lp-home h1{font-size:clamp(2rem,5vw,3.4rem);letter-spacing:-.02em}
#lp-home h2{font-size:clamp(1.6rem,3.4vw,2.3rem);letter-spacing:-.01em}
#lp-home h3{font-size:1.15rem}
#lp-home p{margin:0 0 1em}
#lp-home .lead{font-size:1.2rem;color:var(--muted);max-width:640px}
#lp-home .center{text-align:center}#lp-home .center .lead{margin-left:auto;margin-right:auto}
#lp-home .tc{background:#4a3b00;color:#ffe066;border-radius:4px;padding:0 4px;font-size:.85em;font-weight:600}
:root[data-theme="light"] #lp-home .tc{background:#fff3bf;color:#5c4400}

/* header */
#lp-home header.site{position:sticky;top:0;z-index:10;background:color-mix(in srgb,var(--bg) 92%,transparent);backdrop-filter:blur(8px);border-bottom:1px solid var(--line)}
#lp-home header.site .wrap{display:flex;align-items:center;justify-content:space-between;height:64px}
#lp-home .logo{font-weight:800;font-size:1.3rem;color:var(--ink);text-decoration:none}
#lp-home .logo span{color:var(--brand)}
#lp-home nav.main{display:flex;gap:24px;align-items:center}
#lp-home nav.main a{color:var(--ink);text-decoration:none;font-size:.95rem}
#lp-home nav.main a:hover{color:var(--brand)}
@media(max-width:760px){#lp-home nav.main a:not(.btn){display:none}}

/* buttons */
#lp-home .btn{display:inline-block;background:var(--brand);color:#fff!important;padding:13px 24px;border-radius:10px;font-weight:600;text-decoration:none;border:2px solid var(--brand);transition:.15s}
#lp-home .btn:hover{background:var(--brand-dark);border-color:var(--brand-dark)}
#lp-home .btn.ghost{background:transparent;color:var(--brand)!important}
#lp-home .btn.ghost:hover{background:var(--brand);color:#fff!important}
#lp-home .btn.sm{padding:8px 16px;font-size:.92rem}
#lp-home .cta-row{display:flex;gap:12px;flex-wrap:wrap;margin:24px 0 10px}
#lp-home .micro{font-size:.88rem;color:var(--muted)}

/* image placeholders */
#lp-home .ph{background:repeating-linear-gradient(45deg,var(--ph),var(--ph) 12px,color-mix(in srgb,var(--ph) 70%,var(--bg)) 12px,color-mix(in srgb,var(--ph) 70%,var(--bg)) 24px);
  border:2px dashed var(--ph-line);border-radius:var(--radius);display:flex;align-items:center;justify-content:center;text-align:center;
  color:var(--muted);font-size:.85rem;padding:16px;font-weight:600;min-height:220px}
#lp-home .ph small{display:block;font-weight:400;margin-top:4px}
#lp-home .ph.hero-img{min-height:380px}
#lp-home .ph.logo-ph{min-height:56px;font-size:.75rem;padding:8px}

/* hero */
#lp-home .hero .grid{display:grid;grid-template-columns:1.05fr 1fr;gap:48px;align-items:center}
@media(max-width:900px){#lp-home .hero .grid{grid-template-columns:1fr}}
#lp-home .badge{display:inline-block;font-size:.8rem;font-weight:700;color:var(--brand);background:color-mix(in srgb,var(--brand) 12%,transparent);padding:4px 12px;border-radius:99px;margin-bottom:16px}

/* tool */
#lp-home .tool{background:var(--bg);border:1px solid var(--line);border-radius:var(--radius);padding:28px;max-width:820px;margin:32px auto 0;box-shadow:0 10px 30px rgba(15,27,45,.06)}
#lp-home .tool form{display:grid;grid-template-columns:1fr 1fr auto;gap:12px}
@media(max-width:760px){#lp-home .tool form{grid-template-columns:1fr}}
#lp-home .tool input{width:100%;padding:13px 14px;border:1px solid var(--line);border-radius:10px;font:inherit;background:var(--bg);color:var(--ink)}
#lp-home .tool button{cursor:pointer;font:inherit}
#lp-home .result-list{display:flex;flex-wrap:wrap;gap:8px;margin-top:16px;font-size:.85rem}
#lp-home .result-list span{background:var(--bg-alt);border:1px solid var(--line);border-radius:99px;padding:3px 12px}

/* proof */
#lp-home .stats{display:grid;grid-template-columns:repeat(3,1fr);gap:20px;margin:32px 0}
@media(max-width:700px){#lp-home .stats{grid-template-columns:1fr}}
#lp-home .stat{text-align:center;padding:20px;border:1px solid var(--line);border-radius:var(--radius);background:var(--bg)}
#lp-home .stat b{display:block;font-size:2rem;color:var(--brand)}
#lp-home .logos{display:grid;grid-template-columns:repeat(5,1fr);gap:16px}
@media(max-width:700px){#lp-home .logos{grid-template-columns:repeat(2,1fr)}}

/* cards */
#lp-home .cards{display:grid;grid-template-columns:repeat(auto-fit,minmax(240px,1fr));gap:20px;margin-top:36px}
#lp-home .card{background:var(--bg);border:1px solid var(--line);border-radius:var(--radius);padding:24px}
#lp-home section.alt .card{background:var(--bg)}
#lp-home .card .ic{width:40px;height:40px;border-radius:10px;display:grid;place-items:center;font-weight:800;color:#fff;background:var(--brand);margin-bottom:12px}
#lp-home .card p{margin:0;color:var(--muted);font-size:.97rem}

/* steps */
#lp-home .steps{counter-reset:s;display:grid;grid-template-columns:repeat(3,1fr);gap:24px;margin-top:36px}
@media(max-width:800px){#lp-home .steps{grid-template-columns:1fr}}
#lp-home .step{counter-increment:s}
#lp-home .step::before{content:counter(s);display:grid;place-items:center;width:40px;height:40px;border-radius:50%;background:var(--brand);color:#fff;font-weight:800;margin-bottom:12px}
#lp-home .step .ph{min-height:160px;margin-bottom:16px}

/* split */
#lp-home .split{display:grid;grid-template-columns:1fr 1fr;gap:48px;align-items:center}
#lp-home .split.rev .ph{order:2}
@media(max-width:860px){#lp-home .split{grid-template-columns:1fr}#lp-home .split.rev .ph{order:0}}
#lp-home ul.check{list-style:none;padding:0;margin:16px 0}
#lp-home ul.check li{padding-left:28px;position:relative;margin-bottom:8px}
#lp-home ul.check li::before{content:"✓";position:absolute;left:0;color:var(--accent);font-weight:800}

/* alert mock */
#lp-home .alert-mock{border:1px solid var(--line);border-left:4px solid var(--danger);border-radius:10px;padding:14px 16px;background:var(--bg);font-size:.9rem;margin-top:20px}
#lp-home .alert-mock b{color:var(--danger)}

/* table */
#lp-home .tablewrap{overflow-x:auto;margin-top:32px}
#lp-home table{width:100%;border-collapse:collapse;min-width:560px;background:var(--bg)}
#lp-home th,#lp-home td{padding:14px 16px;border-bottom:1px solid var(--line);text-align:left}
#lp-home thead th{background:var(--brand);color:#fff}
#lp-home tbody th{font-weight:600}

/* pricing */
#lp-home .plans{display:grid;grid-template-columns:repeat(3,1fr);gap:20px;margin-top:36px}
@media(max-width:860px){#lp-home .plans{grid-template-columns:1fr}}
#lp-home .plan{border:1px solid var(--line);border-radius:var(--radius);padding:28px;background:var(--bg);display:flex;flex-direction:column}
#lp-home .plan.hl{border:2px solid var(--brand)}
#lp-home .plan .price{font-size:2.2rem;font-weight:800;margin:8px 0}
#lp-home .plan ul{padding-left:20px;color:var(--muted);flex:1}

/* reviews */
#lp-home .reviews{display:grid;grid-template-columns:repeat(3,1fr);gap:20px;margin-top:36px}
@media(max-width:860px){#lp-home .reviews{grid-template-columns:1fr}}
#lp-home .review{border:1px dashed var(--ph-line);border-radius:var(--radius);padding:24px;background:var(--bg)}
#lp-home .review .who{display:flex;gap:12px;align-items:center;margin-top:16px}
#lp-home .review .avatar{width:48px;height:48px;border-radius:50%;flex:none;min-height:0;padding:0;font-size:.6rem}

/* faq */
#lp-home .faq{max-width:820px;margin:32px auto 0}
#lp-home details{border:1px solid var(--line);border-radius:12px;padding:16px 20px;margin-bottom:12px;background:var(--bg)}
#lp-home summary{cursor:pointer;font-weight:600;list-style:none;display:flex;justify-content:space-between;gap:16px}
#lp-home summary::-webkit-details-marker{display:none}
#lp-home summary::after{content:"+";font-size:1.4rem;line-height:1;color:var(--brand)}
#lp-home details[open] summary::after{content:"–"}
#lp-home details p{margin:12px 0 0;color:var(--muted)}

/* final — fixed dark band (readable in both themes) */
#lp-home .final{background:#111720;color:#fff;text-align:center}
#lp-home .final .lead{color:#b9c6df;margin:0 auto}
#lp-home .final .btn{background:#fff;color:#111720!important;border-color:#fff}

#lp-home footer.site{padding:40px 0;border-top:1px solid var(--line);font-size:.92rem;color:var(--muted)}
#lp-home footer.site .cols{display:flex;flex-wrap:wrap;gap:32px;justify-content:space-between}
#lp-home footer.site a{color:var(--muted);display:block;margin-bottom:6px;text-decoration:none}
#lp-home footer.site a:hover{color:var(--brand)}
</style>

<script type="application/ld+json">
{
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
        "Lost backlink alerts",
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
        "url": "https://www.linkpulz.com/pricing/"
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
}
</script>

<div id="lp-home">
<header class="site">
  <div class="wrap">
    <a class="logo" href="/" aria-label="LinkPulz home">Link<span>Pulz</span></a>
    <nav class="main" aria-label="Main">
      <a href="#features">Features</a>
      <a href="#how-it-works">How it works</a>
      <a href="#pricing">Pricing</a>
      <a href="#faq">FAQ</a>
      <a class="btn sm" href="/signup/">Start free trial</a>
    </nav>
  </div>
</header>

<main>

<!-- ============ HERO ============ -->
<section class="hero" aria-labelledby="h1">
  <div class="wrap grid">
    <div>
      <span class="badge">Backlink monitoring software</span>
      <h1 id="h1">Backlink Monitoring That Alerts You Before a Link Is Lost</h1>
      <p class="lead">LinkPulz checks every backlink you've earned or paid for, and tells you the moment one disappears, turns nofollow, redirects or gets deindexed. Stop finding out three months too late.</p>
      <div class="cta-row">
        <a class="btn" href="/signup/">Start free trial</a>
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
      <form action="/free-backlink-checker/" method="get">
        <input type="url" name="source" placeholder="Source URL (page that links to you)" required aria-label="Source URL">
        <input type="url" name="target" placeholder="Target URL (your page)" required aria-label="Target URL">
        <button class="btn" type="submit">Check my link</button>
      </form>
      <div class="result-list" aria-label="What the checker reports">
        <span>Link found / not found</span><span>HTTP status</span><span>dofollow · nofollow · sponsored · UGC</span><span>Indexed or not</span><span>Anchor text</span>
      </div>
      <p class="micro" style="margin:16px 0 0">Want this checked automatically? <a href="/signup/">Monitor this link free</a> <span class="tc">frequency: TO CONFIRM</span></p>
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
      <div class="card"><div class="ic">✕</div><h3>Lost links</h3><p>The page removes your link or disappears altogether.</p></div>
      <div class="card"><div class="ic">404</div><h3>Broken pages</h3><p>The linking page returns a 404 or 5xx error.</p></div>
      <div class="card"><div class="ic">↪</div><h3>Redirects</h3><p>The linking page now redirects somewhere else.</p></div>
      <div class="card"><div class="ic">rel</div><h3>Attribute changes</h3><p>Dofollow switches to nofollow, sponsored or UGC.</p></div>
      <div class="card"><div class="ic">NI</div><h3>Noindex</h3><p>The linking page is set to noindex and passes little value.</p></div>
      <div class="card"><div class="ic">G</div><h3>Deindexation</h3><p>The linking page drops out of Google's index <span class="tc">TO CONFIRM how detected</span>.</p></div>
      <div class="card"><div class="ic">Aa</div><h3>Anchor text changes</h3><p>Your agreed anchor is edited or replaced.</p></div>
      <div class="card"><div class="ic">+</div><h3>New backlinks</h3><p>Links you didn't track yet show up <span class="tc">TO CONFIRM source</span>.</p></div>
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
      <p>Track each recovery attempt until the link is back <span class="tc">TO CONFIRM workflow feature</span>. See our guide to <a href="/lost-backlinks/">recovering lost backlinks</a>.</p>
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
      <p class="lead">Only publish facts you can verify today. <a href="/alternatives/">See all alternatives</a>.</p>
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
      <div class="plan"><h3>Starter</h3><div class="price"><span class="tc">$X</span><small class="micro">/mo</small></div><ul><li><span class="tc">X</span> links monitored</li><li>Email alerts</li></ul><a class="btn ghost" href="/signup/">Start free trial</a></div>
      <div class="plan hl"><h3>Pro</h3><div class="price"><span class="tc">$X</span><small class="micro">/mo</small></div><ul><li><span class="tc">X</span> links monitored</li><li>Slack &amp; webhook alerts</li><li>Client reports</li></ul><a class="btn" href="/signup/">Start free trial</a></div>
      <div class="plan"><h3>Agency</h3><div class="price"><span class="tc">$X</span><small class="micro">/mo</small></div><ul><li><span class="tc">X</span> links monitored</li><li>Team seats</li><li>White-label reports</li></ul><a class="btn ghost" href="/signup/">Start free trial</a></div>
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
    <div class="cta-row" style="justify-content:center"><a class="btn" href="/signup/">Start free trial</a></div>
  </div>
</section>

</main>

<footer class="site">
  <div class="wrap cols">
    <div><a class="logo" href="/">Link<span>Pulz</span></a><p class="micro" style="margin-top:8px">Backlink monitoring software.</p></div>
    <div><b>Product</b><br><a href="/backlink-alerts/">Backlink alerts</a><a href="/lost-backlinks/">Lost backlinks</a><a href="/free-backlink-checker/">Free backlink checker</a><a href="/pricing/">Pricing</a></div>
    <div><b>Use cases</b><br><a href="/backlink-reporting-for-agencies/">For agencies</a><a href="/guest-post-link-monitoring/">Guest post monitoring</a><a href="/alternatives/">Alternatives</a></div>
    <div><b>Resources</b><br><a href="/blog/nofollow-vs-dofollow/">Nofollow vs dofollow</a><a href="/blog/backlink-audit/">Backlink audit</a><a href="/blog/">Blog</a></div>
  </div>
  <div class="wrap micro" style="margin-top:24px">© <span id="y">2026</span> LinkPulz. All rights reserved.</div>
</footer>
</div>
`;
