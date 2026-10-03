// Shared chrome for the marketing / SEO pages (homepage + the backlink tool
// pages). One source of truth for the LinkPulz charte: the full #lp-home-scoped
// stylesheet, the sticky header and the footer. Each page supplies only its own
// JSON-LD graph and its <main> content via `marketingPage(jsonLd, main)`.
//
// All CSS is scoped under #lp-home so it never collides with the app's global
// stylesheets (globals.css / theme-light.css). Static markup, injected by a
// Server Component via dangerouslySetInnerHTML — zero client JS. The light/dark
// switch rides the app's `data-theme` toggle (not OS prefers-color-scheme).

export const MARKETING_STYLE = `
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
#lp-home ol{padding-left:22px;margin:16px 0}
#lp-home ol li{margin-bottom:10px}
#lp-home .prose{max-width:760px}
#lp-home .prose.center{margin-left:auto;margin-right:auto}

/* breadcrumb */
#lp-home .crumbs{font-size:.85rem;color:var(--muted);padding:18px 0 0}
#lp-home .crumbs a{color:var(--muted)}
#lp-home .crumbs a:hover{color:var(--brand)}
#lp-home .crumbs span{margin:0 6px;opacity:.6}

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
#lp-home .hero{text-align:left;overflow:visible}
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

/* verify tool */
#lp-home .tool.verify form{grid-template-columns:1fr 1fr;gap:14px}
#lp-home .tool.verify form label{display:flex;flex-direction:column;gap:6px;text-align:left;font-size:.9rem;font-weight:600}
#lp-home .tool.verify form button{grid-column:1 / -1}
@media(max-width:620px){#lp-home .tool.verify form{grid-template-columns:1fr}}
#lp-home .verify-result{max-width:420px;margin:20px auto 0;padding:16px 20px;border:1px solid var(--line);border-radius:var(--radius);background:var(--bg-alt);text-align:left}
#lp-home .verify-label{margin:0 0 8px;text-transform:uppercase;letter-spacing:.04em}
#lp-home .verify-result ul.check{margin:0}
#lp-home .verify-cta{margin-top:24px}
#lp-home .verify-cta p{margin-bottom:12px}

/* lost / broken demos */
#lp-home .lost-demo{border:1px solid var(--line);border-radius:var(--radius);background:var(--bg-alt);padding:18px 20px}
#lp-home .lost-row{display:flex;align-items:center;gap:12px;flex-wrap:wrap;padding:10px 0;border-bottom:1px solid var(--line);font-size:.92rem}
#lp-home .lost-row small{color:var(--muted);min-width:72px}
#lp-home .lost-row .url{flex:1;min-width:0;overflow:hidden;text-overflow:ellipsis;white-space:nowrap}
#lp-home .status{font-weight:700;font-size:.8rem;white-space:nowrap}
#lp-home .status.live{color:var(--accent)}
#lp-home .status.lost{color:var(--danger)}
#lp-home .lost-alert{margin-top:14px;border:1px solid color-mix(in srgb,var(--danger) 32%,var(--line));border-radius:10px;padding:12px 14px;font-size:.9rem;background:color-mix(in srgb,var(--danger) 8%,var(--bg))}
#lp-home .lost-alert b{color:var(--danger)}
#lp-home .flow{display:flex;flex-direction:column;align-items:center;gap:6px;margin:14px 0;padding:14px;border:1px solid var(--line);border-radius:10px;background:var(--bg-alt);text-align:center}
#lp-home .flow .url{font-size:.9rem;word-break:break-all}
#lp-home .flow .arrow{color:var(--muted);font-size:1.2rem;line-height:1}
#lp-home .alt-box{border:1px solid color-mix(in srgb,var(--brand) 32%,var(--line));border-radius:var(--radius);background:color-mix(in srgb,var(--brand) 8%,var(--bg-alt));padding:22px 24px}
#lp-home .alt-box .btn{margin-top:8px}

/* health dashboard */
#lp-home .health{max-width:900px;margin:36px auto 0;padding:24px;border:1px solid var(--line);border-radius:var(--radius);background:var(--bg-alt)}
#lp-home .health-top{display:flex;gap:32px;align-items:center;flex-wrap:wrap}
#lp-home .health-score{text-align:center;flex:none}
#lp-home .health-score small{display:block;color:var(--muted);font-size:.78rem;text-transform:uppercase;letter-spacing:.05em}
#lp-home .health-score b{display:block;font-size:3rem;font-weight:800;color:var(--accent);line-height:1.05}
#lp-home .health-score span{color:var(--muted);font-size:.85rem}
#lp-home .health-main{flex:1;min-width:280px}
#lp-home .health-bar{display:flex;height:14px;border-radius:8px;overflow:hidden;margin-bottom:18px;background:var(--bg)}
#lp-home .health-bar i{display:block;height:100%}
#lp-home .health-table{width:100%;border-collapse:collapse;background:transparent;min-width:0}
#lp-home .health-table th{text-align:left;font-weight:500;padding:6px 0;width:100%}
#lp-home .health-table td{text-align:right;padding:6px 0;font-variant-numeric:tabular-nums;white-space:nowrap}
#lp-home .dot{display:inline-block;width:10px;height:10px;border-radius:50%;margin-right:8px}
#lp-home .sr-only{position:absolute;width:1px;height:1px;padding:0;margin:-1px;overflow:hidden;clip:rect(0,0,0,0);white-space:nowrap;border:0}

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
#lp-home .alert-mock{border:1px solid color-mix(in srgb,var(--danger) 32%,var(--line));border-radius:10px;padding:14px 16px;background:color-mix(in srgb,var(--danger) 8%,var(--bg));font-size:.9rem;margin-top:20px}
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

/* ── craft: browser surfaces, focus, numerals, one authored entrance ── */
#lp-home ::selection{background:color-mix(in srgb,var(--brand) 32%,transparent);color:var(--ink)}
#lp-home input{caret-color:var(--brand)}
#lp-home a:not(.btn){text-underline-offset:3px;text-decoration-thickness:1px}
#lp-home a:not(.btn):hover{text-decoration:underline}
#lp-home a:focus-visible,#lp-home button:focus-visible,#lp-home input:focus-visible,#lp-home summary:focus-visible{outline:2px solid var(--brand);outline-offset:3px;border-radius:8px}
#lp-home .tool input:focus{outline:none;border-color:var(--brand);box-shadow:0 0 0 3px color-mix(in srgb,var(--brand) 25%,transparent)}
#lp-home .stat b,#lp-home .health-score b,#lp-home .plan .price,#lp-home .health-table td{font-variant-numeric:tabular-nums}
#lp-home .tablewrap{scrollbar-width:thin;scrollbar-color:var(--line) transparent}
#lp-home .tablewrap::-webkit-scrollbar{height:10px}
#lp-home .tablewrap::-webkit-scrollbar-thumb{background:var(--line);border-radius:99px}

@keyframes lp-rise{from{opacity:0;transform:translateY(14px)}to{opacity:1;transform:none}}
#lp-home .hero .badge,#lp-home .hero h1,#lp-home .hero .lead,#lp-home .hero ul.check,#lp-home .hero .cta-row,#lp-home .hero .micro{animation:lp-rise .7s cubic-bezier(.16,1,.3,1) both}
#lp-home .hero h1{animation-delay:.05s}
#lp-home .hero .lead{animation-delay:.1s}
#lp-home .hero ul.check{animation-delay:.16s}
#lp-home .hero .cta-row{animation-delay:.22s}
#lp-home .hero .micro{animation-delay:.28s}
@media(prefers-reduced-motion:reduce){#lp-home .hero .badge,#lp-home .hero h1,#lp-home .hero .lead,#lp-home .hero ul.check,#lp-home .hero .cta-row,#lp-home .hero .micro{animation:none}}
</style>`;

// Shared sticky header. Nav anchors are absolute (/#…) so they work from any
// marketing page, scrolling to the homepage's sections.
export const MARKETING_HEADER = `
<header class="site">
  <div class="wrap">
    <a class="logo" href="/" aria-label="LinkPulz home"><img src="https://ye8jhaekbbcviws9.public.blob.vercel-storage.com/logo-linkpulz.webp" alt="LinkPulz backlink monitoring software" width="1254" height="1254" style="height:26px;width:auto;margin-right:9px;vertical-align:-6px;display:inline-block">Link<span>Pulz</span></a>
    <nav class="main" aria-label="Main">
      <a href="/#features">Features</a>
      <a href="/#how-it-works">How it works</a>
      <a href="/#pricing">Pricing</a>
      <a href="/#faq">FAQ</a>
      <a class="btn sm" href="/register">Start free trial</a>
    </nav>
  </div>
</header>`;

export const MARKETING_FOOTER = `
<footer class="site">
  <div class="wrap cols">
    <div><a class="logo" href="/">Link<span>Pulz</span></a><p class="micro" style="margin-top:8px">Backlink monitoring software.</p></div>
    <div><b>Product</b><br><a href="/lost-backlinks">Lost backlinks</a><a href="/broken-backlinks">Broken backlinks</a><a href="/verify-backlinks">Verify a backlink</a><a href="/backlink-health-check">Backlink health check</a></div>
    <div><b>Guides</b><br><a href="/backlink-management">Backlink management</a><a href="/backlink-audit">Backlink audit</a><a href="/backlink-profile">Backlink profile</a><a href="/backlink-reporting">Backlink reporting</a></div>
    <div><b>Tools</b><br><a href="/tools/backlink-checker">Free backlink checker</a><a href="/docs">Documentation</a><a href="/#pricing">Pricing</a></div>
    <div><b>Company</b><br><a href="/register">Start free trial</a><a href="/login">Log in</a><a href="/legal/privacy">Privacy</a><a href="/legal/terms">Terms</a></div>
  </div>
  <div class="wrap micro" style="margin-top:24px">© <span id="y">2026</span> LinkPulz. All rights reserved.</div>
</footer>`;

/**
 * Assemble a full marketing page's innerHTML: the shared stylesheet, the page's
 * JSON-LD graph, then the #lp-home wrapper around the shared header, the page's
 * <main> content and the shared footer. The caller injects the result via
 * dangerouslySetInnerHTML and sets <head> metadata through Next's metadata API.
 */
export function marketingPage(jsonLd: string, main: string): string {
  return `${MARKETING_STYLE}

<script type="application/ld+json">
${jsonLd}
</script>

<div id="lp-home">
${MARKETING_HEADER}

<main>
${main}
</main>

${MARKETING_FOOTER}
</div>`;
}
