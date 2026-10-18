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
  --brand:#4c82ff; --brand-dark:#2f6bff; --violet:#9b7cff; --accent:#36d399; --danger:#fb7185; --warn:#fbbf24;
  --ph-line:rgba(255,255,255,.15); --radius:14px; --max:1120px;
  font:17px/1.6 var(--font-sans);
  color:var(--ink);
  background:radial-gradient(circle at 50% -10%,color-mix(in srgb,var(--brand) 18%,transparent),transparent 36%),var(--bg);
  min-height:100dvh;
}
:root[data-theme="light"] #lp-home{
  --bg:#ffffff; --bg-alt:#f4f7fb; --ink:#0f1b2d; --muted:#51607a; --line:#dbe3ee;
  --brand:#2f6bff; --brand-dark:#1e54e6; --violet:#7c5cff; --accent:#12b886; --danger:#e03131; --warn:#f08c00;
  --ph-line:#b7c4d9;
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
#lp-home header.site{position:sticky;top:0;z-index:20;background:color-mix(in srgb,var(--bg) 88%,transparent);backdrop-filter:blur(10px);border-bottom:1px solid var(--line)}
#lp-home header.site .wrap{display:flex;align-items:center;justify-content:space-between;height:64px}
#lp-home .logo{font-weight:800;font-size:1.3rem;color:var(--ink);text-decoration:none}
#lp-home .logo span{color:var(--brand)}
#lp-home nav.main{display:flex;gap:28px;align-items:center}
#lp-home nav.main a:not(.btn){color:var(--ink);text-decoration:none;font-size:.95rem;font-weight:550;position:relative;padding:5px 0;transition:color .15s}
#lp-home nav.main a:not(.btn):hover{color:var(--brand)}
#lp-home nav.main a:not(.btn)::after{content:"";position:absolute;left:0;right:0;bottom:-1px;height:2px;border-radius:2px;background:var(--brand);transform:scaleX(0);transform-origin:left;transition:transform .18s cubic-bezier(.16,1,.3,1)}
#lp-home nav.main a:not(.btn):hover::after{transform:scaleX(1)}
/* mobile menu (pure CSS, no JS): the burger label toggles a hidden checkbox */
#lp-home .nav-toggle{position:absolute;width:1px;height:1px;opacity:0;pointer-events:none}
#lp-home .nav-burger{display:none}
@media(max-width:760px){
  #lp-home .nav-burger{display:inline-flex;flex-direction:column;justify-content:center;gap:5px;width:42px;height:42px;margin-right:-10px;cursor:pointer;border-radius:10px}
  #lp-home .nav-burger span{display:block;width:22px;height:2px;background:var(--ink);border-radius:2px;margin:0 auto;transition:transform .2s ease,opacity .2s ease}
  #lp-home nav.main{position:absolute;top:100%;left:0;right:0;flex-direction:column;align-items:stretch;gap:0;background:var(--bg);border-bottom:1px solid var(--line);box-shadow:0 16px 34px rgba(0,0,0,.28);padding:6px 20px 18px;display:none}
  #lp-home .nav-toggle:checked~nav.main{display:flex;animation:lp-rise .22s ease both}
  #lp-home nav.main a:not(.btn){padding:14px 2px;border-bottom:1px solid var(--line);font-size:1rem}
  #lp-home nav.main a:not(.btn)::after{display:none}
  #lp-home nav.main a.btn{margin-top:14px;justify-content:center;text-align:center}
  #lp-home .nav-toggle:checked~.nav-burger span:nth-child(1){transform:translateY(7px) rotate(45deg)}
  #lp-home .nav-toggle:checked~.nav-burger span:nth-child(2){opacity:0}
  #lp-home .nav-toggle:checked~.nav-burger span:nth-child(3){transform:translateY(-7px) rotate(-45deg)}
}

/* buttons */
#lp-home .btn{display:inline-block;background:linear-gradient(135deg,var(--brand),var(--violet));color:#fff!important;padding:13px 24px;border-radius:10px;font-weight:600;text-decoration:none;border:2px solid transparent;box-shadow:0 6px 18px color-mix(in srgb,var(--brand) 24%,transparent);transition:filter .15s,transform .15s}
#lp-home .btn:hover{filter:brightness(1.05) saturate(1.05)}
#lp-home .btn:active{transform:translateY(1px)}
#lp-home .btn.ghost{background:transparent;color:var(--brand)!important;border-color:var(--brand);box-shadow:none}
#lp-home .btn.ghost:hover{background:var(--brand);color:#fff!important}
#lp-home .btn.sm{padding:8px 16px;font-size:.92rem}
#lp-home .cta-row{display:flex;gap:12px;flex-wrap:wrap;margin:24px 0 10px}
#lp-home .micro{font-size:.88rem;color:var(--muted)}


/* product shots — inline UI mockups built from the real design language */
#lp-home .shot{border:1px solid var(--line);border-radius:var(--radius);background:var(--bg);overflow:hidden;box-shadow:0 12px 34px rgba(15,27,45,.10);font-size:.8rem;color:var(--ink)}
#lp-home .shot-bar{display:flex;align-items:center;gap:9px;height:34px;padding:0 13px;background:var(--bg-alt);border-bottom:1px solid var(--line)}
#lp-home .shot-dots{display:flex;gap:5px}
#lp-home .shot-dots i{width:9px;height:9px;border-radius:50%;background:var(--ph-line);display:block}
#lp-home .shot-ttl{font-size:.72rem;color:var(--muted);font-weight:600}
#lp-home .shot-body{padding:14px 16px;display:flex;flex-direction:column;gap:11px}
#lp-home .shot .ex{align-self:flex-start;font-size:.6rem;text-transform:uppercase;letter-spacing:.06em;color:var(--muted);font-weight:700;background:var(--bg-alt);border:1px solid var(--line);border-radius:99px;padding:2px 8px}
#lp-home .shot .cap{font-size:.68rem;font-weight:700;text-transform:uppercase;letter-spacing:.05em;color:var(--muted);margin-bottom:6px}
#lp-home .srow{display:flex;align-items:center;gap:10px;padding:7px 0;border-bottom:1px solid var(--line)}
#lp-home .srow:last-child{border-bottom:0}
#lp-home .surl{flex:1;min-width:0;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;font-size:.76rem}
#lp-home .smuted{color:var(--muted)}
#lp-home .sbadge{font-size:.66rem;font-weight:700;padding:2px 8px;border-radius:99px;white-space:nowrap}
#lp-home .sbadge.live{color:var(--accent);background:color-mix(in srgb,var(--accent) 15%,transparent)}
#lp-home .sbadge.lost{color:var(--danger);background:color-mix(in srgb,var(--danger) 13%,transparent)}
#lp-home .sbadge.changed{color:var(--warn);background:color-mix(in srgb,var(--warn) 18%,transparent)}
#lp-home .stags{display:flex;flex-wrap:wrap;gap:6px;align-items:center}
#lp-home .stag{font-size:.66rem;border:1px solid var(--line);border-radius:99px;padding:2px 9px;color:var(--muted);background:var(--bg-alt)}
#lp-home .sbtn{display:inline-block;font-size:.72rem;font-weight:650;padding:7px 12px;border-radius:8px;background:linear-gradient(135deg,var(--brand),var(--violet));color:#fff}
#lp-home .sbtn.ghost{background:var(--bg-alt);color:var(--ink);border:1px solid var(--line)}
#lp-home .stiles{display:grid;grid-template-columns:repeat(3,1fr);gap:8px}
#lp-home .stile{border:1px solid var(--line);border-radius:8px;padding:9px 10px;background:var(--bg-alt)}
#lp-home .stile b{display:block;font-size:1.05rem}
#lp-home .stile small{color:var(--muted);font-size:.62rem}
#lp-home .smail{border:1px solid var(--line);border-radius:10px;overflow:hidden}
#lp-home .smail-h{padding:10px 12px;background:var(--bg-alt);border-bottom:1px solid var(--line)}
#lp-home .smail-b{padding:11px 12px;font-size:.74rem}
#lp-home .sdiff{font-size:.73rem;color:var(--muted);line-height:1.6}
#lp-home .sdiff del{color:var(--danger)}
#lp-home .sdiff ins{color:var(--ink);text-decoration:none;font-weight:600}
#lp-home .stl{list-style:none;margin:0;padding:0;font-size:.72rem}
#lp-home .stl li{position:relative;padding:0 0 9px 15px;border-left:1px solid var(--line);margin-left:4px}
#lp-home .stl li:last-child{border-left-color:transparent;padding-bottom:0}
#lp-home .stl li::before{content:"";position:absolute;left:-4px;top:3px;width:7px;height:7px;border-radius:50%;background:var(--brand)}
#lp-home .step .shot{margin-bottom:16px}
#lp-home .split.rev .shot{order:2}
@media(max-width:860px){#lp-home .split.rev .shot{order:0}}

/* hero — the headline leads, so give it a wider column and a larger, tighter
   display size than the shared h1 step (a stronger first-screen focal point). */
#lp-home .hero{text-align:left;overflow:visible}
#lp-home .hero .grid{display:grid;grid-template-columns:1.2fr .9fr;gap:48px;align-items:center}
#lp-home .hero h1{font-size:clamp(2.3rem,6vw,4.2rem);line-height:1.04;letter-spacing:-.03em}
@media(max-width:900px){#lp-home .hero .grid{grid-template-columns:1fr}}
#lp-home .badge{display:inline-block;font-size:.8rem;font-weight:700;color:var(--brand);background:color-mix(in srgb,var(--brand) 12%,transparent);padding:4px 12px;border-radius:99px;margin-bottom:16px}

/* tool */
#lp-home .tool{background:var(--bg);border:1px solid var(--line);border-radius:var(--radius);padding:28px;max-width:820px;margin:32px auto 0;box-shadow:0 10px 30px rgba(15,27,45,.06)}
#lp-home .tool form{display:grid;grid-template-columns:1fr 1fr auto;gap:12px}
@media(max-width:760px){#lp-home .tool form{grid-template-columns:1fr}}
#lp-home .tool input{width:100%;padding:13px 14px;border:1px solid var(--line);border-radius:10px;font:inherit;background:var(--bg);color:var(--ink)}
#lp-home .tool button{cursor:pointer;font:inherit}

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

/* split */
#lp-home .split{display:grid;grid-template-columns:1fr 1fr;gap:48px;align-items:center}
@media(max-width:860px){#lp-home .split{grid-template-columns:1fr}}
#lp-home ul.check{list-style:none;padding:0;margin:16px 0}
#lp-home ul.check li{padding-left:28px;position:relative;margin-bottom:8px}
#lp-home ul.check li::before{content:"✓";position:absolute;left:0;color:var(--accent);font-weight:800}


/* table */
#lp-home .tablewrap{overflow-x:auto;margin-top:32px}
#lp-home table{width:100%;border-collapse:collapse;min-width:560px;background:var(--bg)}
#lp-home th,#lp-home td{padding:14px 16px;border-bottom:1px solid var(--line);text-align:left}
#lp-home thead th{background:var(--brand);color:#fff}
#lp-home tbody th{font-weight:600}

/* comparison table: mute competitors, highlight the LinkPulz column */
#lp-home table.compare{min-width:760px}
#lp-home table.compare thead th{background:var(--bg-alt);color:var(--ink);font-weight:600;vertical-align:bottom}
#lp-home table.compare thead th.us{background:var(--brand);color:#fff}
#lp-home table.compare td{color:var(--muted);font-size:.93rem}
#lp-home table.compare td.us{color:var(--ink);background:color-mix(in srgb,var(--brand) 10%,transparent);font-weight:500}
#lp-home table.compare th.us,#lp-home table.compare td.us{border-left:1px solid color-mix(in srgb,var(--brand) 35%,var(--line));border-right:1px solid color-mix(in srgb,var(--brand) 35%,var(--line))}
#lp-home table.compare tbody tr:last-child td.us{border-bottom:1px solid color-mix(in srgb,var(--brand) 35%,var(--line))}

/* pricing */
#lp-home .plans{display:grid;grid-template-columns:repeat(3,1fr);gap:20px;margin-top:36px}
@media(max-width:860px){#lp-home .plans{grid-template-columns:1fr}}
#lp-home .plan{border:1px solid var(--line);border-radius:var(--radius);padding:28px;background:var(--bg);display:flex;flex-direction:column}
#lp-home .plan.hl{border:2px solid var(--brand)}
#lp-home .plan .price{font-size:2.2rem;font-weight:800;margin:8px 0}
#lp-home .plan ul{padding-left:20px;color:var(--muted);flex:1}


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
#lp-home .final .btn{background:#fff;color:#111720!important;border-color:#fff;box-shadow:none}

#lp-home footer.site{padding:52px 0 40px;border-top:1px solid var(--line);font-size:.92rem;color:var(--muted)}
#lp-home footer.site .cols{display:grid;grid-template-columns:1.6fr 1fr 1fr 1fr 1fr;gap:28px 32px}
@media(max-width:860px){#lp-home footer.site .cols{grid-template-columns:1.4fr 1fr 1fr}}
@media(max-width:560px){#lp-home footer.site .cols{grid-template-columns:1fr 1fr}}
#lp-home footer.site .cols>div>b{display:block;color:var(--ink);font-weight:650;font-size:.82rem;letter-spacing:.01em;margin-bottom:12px}
#lp-home footer.site a{color:var(--muted);display:block;margin-bottom:9px;text-decoration:none;text-underline-offset:3px;transition:color .14s}
#lp-home footer.site a:hover{color:var(--brand);text-decoration:underline}
#lp-home footer.site .cols .logo{margin-bottom:0}
#lp-home footer.site .cols .logo:hover{text-decoration:none}
/* footer newsletter */
#lp-home .nl{display:flex;flex-wrap:wrap;gap:16px 32px;align-items:center;justify-content:space-between;padding-bottom:28px;margin-bottom:28px;border-bottom:1px solid var(--line)}
#lp-home .nl-copy b{display:block;color:var(--ink);font-size:1rem;margin-bottom:2px}
#lp-home .nl-copy span{font-size:.9rem;color:var(--muted)}
#lp-home .nl-slot{min-width:280px}
#lp-home .nl-form{display:flex;gap:8px;flex-wrap:wrap;align-items:center}
#lp-home .nl-form input{padding:11px 13px;border:1px solid var(--line);border-radius:10px;background:var(--bg);color:var(--ink);font:inherit;font-size:.92rem;min-width:220px;flex:1}
#lp-home .nl-form input:focus{outline:none;border-color:var(--brand);box-shadow:0 0 0 3px color-mix(in srgb,var(--brand) 18%,transparent)}
#lp-home .nl-form button{cursor:pointer;font:inherit;font-weight:600;font-size:.92rem;padding:11px 20px;border-radius:10px;border:0;color:#fff;background:linear-gradient(135deg,var(--brand),var(--violet))}
#lp-home .nl-form button:disabled{opacity:.6;cursor:default}
#lp-home .nl-ts{flex-basis:100%}
#lp-home .nl-error{color:var(--danger);font-size:.85rem;flex-basis:100%;margin:4px 0 0}
#lp-home .nl-done{color:var(--accent);font-weight:600;font-size:.95rem;margin:0}

/* ── craft: browser surfaces, focus, numerals, one authored entrance ── */
#lp-home ::selection{background:color-mix(in srgb,var(--brand) 32%,transparent);color:var(--ink)}
#lp-home input{caret-color:var(--brand)}
#lp-home a:not(.btn){text-underline-offset:3px;text-decoration-thickness:1px}
#lp-home a:not(.btn):hover{text-decoration:underline}
#lp-home a:focus-visible,#lp-home button:focus-visible,#lp-home input:focus-visible,#lp-home summary:focus-visible{outline:2px solid var(--brand);outline-offset:3px;border-radius:8px}
#lp-home .tool input:focus{outline:none;border-color:var(--brand);box-shadow:0 0 0 3px color-mix(in srgb,var(--brand) 25%,transparent)}
#lp-home .health-score b,#lp-home .plan .price,#lp-home .health-table td{font-variant-numeric:tabular-nums}
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

/* ── pricing page (pure-CSS monthly/annual toggle) ── */
#lp-home .bill-radio{position:absolute;width:1px;height:1px;opacity:0;pointer-events:none}
#lp-home .bill-toggle{display:inline-flex;align-items:center;gap:4px;margin:20px auto 6px;padding:5px;border:1px solid var(--line);border-radius:999px;background:var(--bg-alt)}
#lp-home .bill-toggle .seg{font-size:.9rem;font-weight:650;color:var(--muted);padding:8px 18px;border-radius:999px;cursor:pointer;transition:.15s;display:inline-flex;align-items:center;gap:8px;user-select:none}
#lp-home #bill-m:checked~.bill-toggle label[for="bill-m"],#lp-home #bill-a:checked~.bill-toggle label[for="bill-a"]{background:var(--brand);color:#fff}
#lp-home .bill-radio:focus-visible~.bill-toggle{outline:2px solid var(--brand);outline-offset:2px}
#lp-home .save-badge{display:inline-block;font-size:.76rem;font-weight:700;color:var(--accent);background:color-mix(in srgb,var(--accent) 14%,transparent);padding:3px 10px;border-radius:999px}
#lp-home #bill-a:checked~.bill-toggle label[for="bill-a"] .save-badge{background:rgba(255,255,255,.22);color:#fff}
#lp-home .plan .price-annual{display:none}
#lp-home #bill-a:checked~.plans .price-monthly{display:none}
#lp-home #bill-a:checked~.plans .price-annual{display:block}
#lp-home .plans{grid-template-columns:repeat(auto-fit,minmax(232px,1fr))}
#lp-home .plan{position:relative;text-align:left}
#lp-home .plan.hl{box-shadow:0 10px 36px color-mix(in srgb,var(--brand) 20%,transparent)}
#lp-home .plan .pop{position:absolute;top:-12px;left:50%;transform:translateX(-50%);font-size:.72rem;font-weight:700;letter-spacing:.04em;text-transform:uppercase;color:#fff;background:linear-gradient(135deg,var(--brand),var(--violet));padding:4px 13px;border-radius:999px;white-space:nowrap;box-shadow:0 4px 12px color-mix(in srgb,var(--brand) 30%,transparent)}
#lp-home .plan h3{font-size:1.15rem;margin-bottom:2px}
#lp-home .plan .pos{font-size:.88rem;color:var(--muted);min-height:40px;margin-bottom:6px}
#lp-home .plan .price{font-size:2.3rem;font-weight:800;margin:4px 0 0;letter-spacing:-.02em}
#lp-home .plan .price small{font-size:1rem;color:var(--muted);font-weight:500}
#lp-home .plan .per{font-size:.84rem;color:var(--muted);min-height:18px;margin-bottom:14px}
#lp-home .plan .limits{display:grid;gap:5px;margin:0 0 14px;font-size:.92rem}
#lp-home .plan .limits b{font-weight:700}
#lp-home .plan .feats{list-style:none;padding:0;margin:4px 0 16px;flex:1;display:grid;gap:9px}
#lp-home .plan .feats li{padding-left:24px;position:relative;font-size:.9rem;line-height:1.35}
#lp-home .plan .feats li::before{content:"✓";position:absolute;left:0;top:0;color:var(--accent);font-weight:800}
#lp-home .plan .btn{justify-content:center;text-align:center}
#lp-home .price-note{font-size:.9rem;color:var(--muted);margin-top:18px}
</style>`;

// Shared sticky header. Nav anchors are absolute (/#…) so they work from any
// marketing page, scrolling to the homepage's sections.
export const MARKETING_HEADER = `
<header class="site">
  <div class="wrap">
    <a class="logo" href="/" aria-label="LinkPulz home"><img src="https://ye8jhaekbbcviws9.public.blob.vercel-storage.com/Logo_LinkPulz_en_n%C3%A9on_d%C3%A9grad%C3%A9.png" alt="LinkPulz backlink monitoring software" title="LinkPulz" width="866" height="288" style="height:28px;width:auto;display:block"></a>
    <input type="checkbox" id="lp-nav-toggle" class="nav-toggle">
    <label for="lp-nav-toggle" class="nav-burger" aria-label="Open menu"><span></span><span></span><span></span></label>
    <nav class="main" aria-label="Main">
      <a href="/#features">Features</a>
      <a href="/#how-it-works">How it works</a>
      <a href="/pricing">Pricing</a>
      <a href="/#faq">FAQ</a>
      <a class="btn sm" href="/register">Start free trial</a>
    </nav>
  </div>
</header>`;

export const MARKETING_FOOTER = `
<footer class="site">
  <div class="wrap nl">
    <div class="nl-copy"><b>Stay in the loop</b><span>Backlink monitoring tips and launch news. No spam, unsubscribe anytime.</span></div>
    <div id="lp-newsletter" class="nl-slot"></div>
  </div>
  <div class="wrap cols">
    <div><a class="logo" href="/" aria-label="LinkPulz home"><img src="https://ye8jhaekbbcviws9.public.blob.vercel-storage.com/Logo_LinkPulz_en_n%C3%A9on_d%C3%A9grad%C3%A9.png" alt="LinkPulz backlink monitoring software" title="LinkPulz" width="866" height="288" style="height:26px;width:auto;display:block"></a><p class="micro" style="margin-top:8px">Backlink monitoring software.</p><a href="https://www.linkedin.com/showcase/linkpulz" target="_blank" rel="noopener noreferrer me" aria-label="LinkPulz on LinkedIn" style="display:inline-flex;align-items:center;gap:6px;margin-top:4px"><svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor" aria-hidden="true" style="flex:none"><path d="M20.45 20.45h-3.56v-5.57c0-1.33-.02-3.04-1.85-3.04-1.85 0-2.14 1.45-2.14 2.94v5.67H9.35V9h3.42v1.56h.05c.47-.9 1.64-1.85 3.37-1.85 3.6 0 4.27 2.37 4.27 5.46v6.28zM5.34 7.43a2.06 2.06 0 1 1 0-4.13 2.06 2.06 0 0 1 0 4.13zM7.12 20.45H3.55V9h3.57v11.45zM22.23 0H1.77C.79 0 0 .77 0 1.73v20.54C0 23.22.79 24 1.77 24h20.46c.98 0 1.77-.78 1.77-1.73V1.73C24 .77 23.21 0 22.23 0z"/></svg>LinkedIn</a></div>
    <div><b>Product</b><a href="/lost-backlinks">Lost backlinks</a><a href="/broken-backlinks">Broken backlinks</a><a href="/verify-backlinks">Verify a backlink</a><a href="/backlink-health-check">Backlink health check</a></div>
    <div><b>Guides</b><a href="/backlink-management">Backlink management</a><a href="/backlink-audit">Backlink audit</a><a href="/backlink-profile">Backlink profile</a><a href="/backlink-reporting">Backlink reporting</a></div>
    <div><b>Tools</b><a href="/tools/backlink-checker">Free backlink checker</a><a href="/docs">Documentation</a><a href="/pricing">Pricing</a></div>
    <div><b>Company</b><a href="/register">Start free trial</a><a href="/login">Log in</a><a href="/legal">Legal</a><a href="/legal/privacy">Privacy</a><a href="/legal/terms">Terms</a><a href="/legal/notice">Legal notice</a></div>
  </div>
  <div class="wrap micro" style="margin-top:24px">© <span id="y">2026</span> LinkPulz, a service of MeedWise (SASU), Paris, France. All rights reserved.</div>
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
