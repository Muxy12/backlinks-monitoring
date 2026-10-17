// Marketing homepage (§48). Content/structure follow the approved SEO content
// brief (full "backlink monitoring" narrative: what/why/how, feature deep-dives,
// a self-assessment, checker-vs-monitor + competitor comparisons, transparency,
// pricing and FAQ). Every placeholder in the brief is resolved against the
// shipped product — 10-day free trial, no card; checks on a schedule you set
// (daily/weekly/bi-weekly/monthly); email/Slack/webhook alerts; CSV import and
// export; competitor gap analysis; plans $29–$199. No invented testimonials or
// case studies. The six+ product shots are inline `.shot` UI mockups built from
// the real design language (status colors, distribution bars), each labelled
// "Example". The LinkPulz charte (dark + purple, #lp-home-scoped CSS), the sticky
// header and the footer live in ./marketing.ts and are shared with the backlink
// tool pages; this file supplies only the homepage's own JSON-LD graph, its small
// quiz-only <style> block and its <main> sections. The visible FAQ is kept in
// lockstep with the FAQPage JSON-LD below.
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
        "https://www.linkedin.com/showcase/linkpulz",
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
      "name": "Backlink Monitoring Tool with Automated Alerts | LinkPulz",
      "description": "Monitor new and lost backlinks, track dofollow and nofollow changes, analyze competitor backlinks and get automated alerts when your links change.",
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
      "description": "Backlink monitoring software that tracks new and lost backlinks, checks your earned and paid links on a schedule, detects dofollow/nofollow and anchor changes, analyzes competitor backlinks and sends automated alerts.",
      "url": "https://www.linkpulz.com/",
      "image": "https://ye8jhaekbbcviws9.public.blob.vercel-storage.com/banniere-dashboard-linkpulz.webp",
      "publisher": { "@id": "https://www.linkpulz.com/#organization" },
      "featureList": [
        "Track new backlinks",
        "Detect lost backlinks",
        "Find broken backlinks",
        "Monitor backlink health",
        "Dofollow, nofollow, sponsored and UGC attribute tracking",
        "Noindex and deindexation detection",
        "Anchor text change detection",
        "Competitor backlink gap analysis",
        "CSV import and export",
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
          "name": "What is backlink monitoring?",
          "acceptedAnswer": { "@type": "Answer", "text": "Backlink monitoring is the ongoing process of tracking the links pointing to your site to identify new backlinks, lost links and changes to existing links. Unlike a one-time check, it keeps a history and alerts you when a relevant change is detected, so you can investigate it." }
        },
        {
          "@type": "Question",
          "name": "How do I monitor my backlinks?",
          "acceptedAnswer": { "@type": "Answer", "text": "Add the links or website you want to track to LinkPulz by pasting them, uploading a CSV or importing referring domains from Google Search Console. Set your check schedule and alert preferences, then review detected changes from the dashboard. Automated alerts keep you informed without checking each link by hand." }
        },
        {
          "@type": "Question",
          "name": "How often does LinkPulz check my backlinks?",
          "acceptedAnswer": { "@type": "Answer", "text": "On a schedule you set: daily, weekly, bi-weekly or monthly, on every plan. Every check is logged in the link's history." }
        },
        {
          "@type": "Question",
          "name": "Can I receive alerts when a backlink disappears?",
          "acceptedAnswer": { "@type": "Answer", "text": "Yes. LinkPulz sends automated alerts by email, Slack or webhook when a monitored link is lost, redirected, nofollowed, deindexed or has its anchor changed. Each alert shows the source URL, your target URL, what changed and when it was detected." }
        },
        {
          "@type": "Question",
          "name": "How can I detect lost backlinks?",
          "acceptedAnswer": { "@type": "Answer", "text": "LinkPulz compares the current observed state of each linking page with what it recorded before. When a previously monitored link is no longer found, it flags the change with the likely cause so you can investigate and decide whether to win it back." }
        },
        {
          "@type": "Question",
          "name": "Can LinkPulz detect when a link changes from dofollow to nofollow?",
          "acceptedAnswer": { "@type": "Answer", "text": "Yes. We track rel attributes (nofollow, sponsored, UGC) on every check and alert you when they change." }
        },
        {
          "@type": "Question",
          "name": "Can I analyze competitor backlinks?",
          "acceptedAnswer": { "@type": "Answer", "text": "Yes. LinkPulz includes competitor backlink gap analysis, which surfaces referring domains that link to competing sites but not yet to yours, so you can turn them into outreach prospects." }
        },
        {
          "@type": "Question",
          "name": "What is the difference between a backlink checker and a backlink monitor?",
          "acceptedAnswer": { "@type": "Answer", "text": "A backlink checker gives you a one-time snapshot of links at a given moment. A backlink monitor tracks the same links over time, keeps their history and alerts you when something changes." }
        },
        {
          "@type": "Question",
          "name": "Does losing a backlink hurt SEO?",
          "acceptedAnswer": { "@type": "Answer", "text": "Losing a backlink does not automatically cause a ranking decline. Its impact depends on the link's relevance, the source page and your broader backlink profile. Monitoring helps you spot the change and decide whether it needs follow-up." }
        },
        {
          "@type": "Question",
          "name": "Is backlink monitoring free?",
          "acceptedAnswer": { "@type": "Answer", "text": "LinkPulz offers a 10-day free trial, with no credit card required, so you can evaluate its monitoring before choosing a paid plan. Paid plans start at $29/mo based on the number of backlinks monitored. See the pricing page for current details." }
        },
        {
          "@type": "Question",
          "name": "Can I manage backlinks using CSV files?",
          "acceptedAnswer": { "@type": "Answer", "text": "Yes. You can import backlinks from a CSV or Excel file and export every monitored link with its full change history to CSV at any time." }
        }
      ]
    }
  ]
}`;

// Hero (LCP) image served through Next's image optimizer: Vercel re-encodes and
// resizes it on the fly, so mobile downloads a right-sized file instead of the
// full 1448×1086 original. The blob host is allow-listed in next.config images.
const HERO_BLOB =
  "https://ye8jhaekbbcviws9.public.blob.vercel-storage.com/banniere-dashboard-linkpulz.webp";
const heroOpt = (w: number) => `/_next/image?url=${encodeURIComponent(HERO_BLOB)}&w=${w}&q=75`;
export const HERO_SIZES = "(max-width: 900px) 92vw, 540px";
export const HERO_SRCSET = [640, 828, 1080, 1200].map((w) => `${heroOpt(w)} ${w}w`).join(", ");
export const HERO_SRC = heroOpt(1200);

// Quiz-only styles live here (not in the shared marketing.ts) so this rewrite
// doesn't collide with other open PRs that touch marketing.ts. A <style> injected
// via dangerouslySetInnerHTML is parsed and applied by the browser (a <script>
// would not execute). Scoped under #lp-home like the rest of the charte.
const QUIZ_STYLE = `<style>
#lp-home .quiz{max-width:880px;margin:36px auto 0}
#lp-home .quiz-q{list-style:none;padding:0;margin:0;display:grid;gap:16px}
#lp-home .quiz-q>li{border:1px solid var(--line);border-radius:var(--radius);background:var(--bg);padding:18px 20px}
#lp-home .quiz-ttl{font-weight:650;margin:0 0 12px}
#lp-home .quiz-opts{display:grid;gap:8px}
#lp-home .quiz-opts label{display:flex;align-items:center;gap:10px;padding:10px 13px;border:1px solid var(--line);border-radius:10px;cursor:pointer;font-size:.95rem;transition:border-color .12s,background .12s}
#lp-home .quiz-opts label:hover{border-color:var(--brand)}
#lp-home .quiz-opts input{accent-color:var(--brand);width:16px;height:16px;flex:none;margin:0}
#lp-home .quiz-opts label:has(input:checked){border-color:var(--brand);background:color-mix(in srgb,var(--brand) 9%,transparent)}
#lp-home .quiz-opts label:has(input:checked) span{font-weight:600}
#lp-home .quiz-res{margin-top:30px}
#lp-home .quiz-res .cap{text-align:center;font-size:.72rem;font-weight:700;text-transform:uppercase;letter-spacing:.06em;color:var(--muted);margin-bottom:4px}
#lp-home .quiz-res .card h3{display:flex;align-items:center;gap:9px}
#lp-home .quiz-res .tier{width:10px;height:10px;border-radius:50%;flex:none;display:inline-block}
#lp-home .quiz-res .next{color:var(--ink);font-size:.9rem;margin:10px 0 0}
#lp-home .quiz-res .next b{color:var(--brand)}
/* Quiz behaviour (driven by the page.tsx inline script). The script runs as the
   parser reaches it, right after the injected body, so it sets up before paint;
   the dangerouslySetInnerHTML subtree is opaque to React's reconciler, so toggling
   classes inside it does not affect hydration. Two layers:
   - Stepper: .stepper shows one question at a time (.active) with a progress bar
     and a Back button; without JS every question shows as a plain scrollable form.
   - Scoring: once all five answers are in, .done reveals the matched profile and
     the verdict. */
#lp-home .quiz-verdict{display:none;text-align:center;font-weight:750;font-size:1.2rem;letter-spacing:-.01em;margin:0 0 20px;color:var(--ink)}
#lp-home .quiz-verdict b{color:var(--brand)}
#lp-home .quiz.done .quiz-res .cap{display:none}
#lp-home .quiz.done .quiz-verdict{display:block;animation:lp-rise .5s cubic-bezier(.16,1,.3,1) both}
#lp-home .quiz.done .quiz-res .cards{grid-template-columns:minmax(0,520px);justify-content:center}
#lp-home .quiz.done .quiz-res .cards .card:not(.matched){display:none}
#lp-home .quiz-res .card.matched{border-color:var(--brand);box-shadow:0 12px 34px color-mix(in srgb,var(--brand) 20%,transparent)}
/* Stepper: one question at a time */
#lp-home .quiz-progress{display:none;align-items:center;gap:14px;margin-bottom:20px}
#lp-home .quiz-step{font-size:.82rem;font-weight:650;color:var(--muted);white-space:nowrap;font-variant-numeric:tabular-nums}
#lp-home .quiz-bar{flex:1;height:6px;border-radius:999px;background:var(--bg-alt);overflow:hidden}
#lp-home .quiz-bar i{display:block;height:100%;border-radius:999px;background:linear-gradient(90deg,var(--brand),var(--violet));transition:width .35s cubic-bezier(.16,1,.3,1)}
#lp-home .quiz-nav{display:none;margin-top:16px}
#lp-home .quiz-back{font:inherit;font-size:.9rem;font-weight:600;color:var(--muted);background:none;border:0;cursor:pointer;padding:6px 4px}
#lp-home .quiz-back:hover{color:var(--brand)}
#lp-home .quiz-restart{font:inherit;font-size:.9rem;font-weight:600;color:var(--brand);background:none;border:0;cursor:pointer;padding:6px 4px;display:none}
#lp-home .quiz-restart:hover{text-decoration:underline}
#lp-home .quiz.stepper .quiz-progress{display:flex}
#lp-home .quiz.stepper .quiz-nav{display:flex}
#lp-home .quiz.stepper .quiz-q{gap:0}
#lp-home .quiz.stepper .quiz-q>li{display:none}
#lp-home .quiz.stepper .quiz-q>li.active{display:block;animation:lp-rise .35s cubic-bezier(.16,1,.3,1) both}
#lp-home .quiz.stepper .quiz-q>li:focus{outline:none}
#lp-home .quiz.stepper[data-step="0"] .quiz-back{visibility:hidden}
#lp-home .quiz.stepper .quiz-res{display:none}
#lp-home .quiz.stepper:not(.done) .quiz-note,#lp-home .quiz.stepper:not(.done) .quiz-cta{display:none}
#lp-home .quiz.stepper.done .quiz-q,#lp-home .quiz.stepper.done .quiz-progress,#lp-home .quiz.stepper.done .quiz-nav{display:none}
#lp-home .quiz.stepper.done .quiz-res{display:block}
#lp-home .quiz.stepper.done .quiz-restart{display:inline-block}
@media(prefers-reduced-motion:reduce){#lp-home .quiz.done .quiz-verdict,#lp-home .quiz.stepper .quiz-q>li.active{animation:none}}
/* Four-up how-it-works steps. A stylesheet rule (not an inline style, which would
   beat the base .steps mobile media query) so the grid collapses on small screens:
   4 → 2 → 1. */
#lp-home .steps.four{grid-template-columns:repeat(4,1fr)}
@media(max-width:900px){#lp-home .steps.four{grid-template-columns:repeat(2,1fr)}}
@media(max-width:560px){#lp-home .steps.four{grid-template-columns:1fr}}
/* Interaction motion: cards lift on hover (brand-safe feedback the skill's motion
   pass asks for). Transform + opacity only, and no travel under reduced motion. */
#lp-home .card{transition:border-color .16s,transform .16s,box-shadow .16s}
#lp-home .card:hover{border-color:color-mix(in srgb,var(--brand) 38%,var(--line));transform:translateY(-3px);box-shadow:0 14px 34px color-mix(in srgb,var(--brand) 12%,transparent)}
@media(prefers-reduced-motion:reduce){#lp-home .card{transition:none}#lp-home .card:hover{transform:none;box-shadow:none}}
/* Scroll-triggered section entrances. The page.tsx MOTION_SCRIPT adds .reveal to
   each non-hero section's parts (before paint, so no flash) and .in once the part
   enters the viewport. Safety by construction: the hiding rule lives ONLY inside
   prefers-reduced-motion:no-preference, and .reveal is only ever added by the
   script, so with no JS or with motion reduced nothing is hidden. .reveal.in (two
   classes) cleanly outranks .reveal (one), and the script's scroll sweep reveals
   anything scrolled past, so content can never stay hidden. */
@media(prefers-reduced-motion:no-preference){
  #lp-home .reveal{opacity:0;transform:translateY(24px);transition:opacity .6s cubic-bezier(.16,1,.3,1),transform .6s cubic-bezier(.16,1,.3,1)}
  #lp-home .reveal.in{opacity:1;transform:none}
}
/* Five-up card rows (the "why" and "who" sections): keep all five on one line on
   desktop, then collapse 5 -> 3 -> 2 -> 1 as the viewport narrows. */
#lp-home .cards.five{grid-template-columns:repeat(5,1fr);gap:16px}
#lp-home .cards.five .card{padding:20px 18px}
@media(max-width:1000px){#lp-home .cards.five{grid-template-columns:repeat(3,1fr)}}
@media(max-width:680px){#lp-home .cards.five{grid-template-columns:repeat(2,1fr)}}
@media(max-width:460px){#lp-home .cards.five{grid-template-columns:1fr}}
/* ===== Live "pulse" hero monitor — the Pulz heartbeat made literal, in pure CSS
   (no JS, so the hero stays edge-cacheable). An ECG line sweeps while monitored
   links tick; on a 9s loop one link flips Live -> Lost and an alert slides in.
   Reduced motion freezes it on a calm, all-live state. ===== */
#lp-home .pulse-card{position:relative;border:1px solid var(--line);border-radius:18px;background:linear-gradient(180deg,color-mix(in srgb,var(--brand) 10%,var(--bg)),var(--bg));box-shadow:0 30px 70px -22px color-mix(in srgb,var(--brand) 45%,transparent);padding:18px 18px 20px;overflow:hidden}
#lp-home .pulse-card::before{content:"";position:absolute;inset:0;background:radial-gradient(120% 60% at 82% -12%,color-mix(in srgb,var(--violet) 24%,transparent),transparent 60%);pointer-events:none}
#lp-home .pulse-top{display:flex;align-items:center;justify-content:space-between;position:relative}
#lp-home .pulse-live{display:inline-flex;align-items:center;gap:9px;font-size:.84rem;font-weight:700;color:var(--ink);letter-spacing:.01em}
#lp-home .pulse-ping{position:relative;width:9px;height:9px;border-radius:50%;background:var(--accent)}
#lp-home .pulse-ping::after{content:"";position:absolute;inset:-4px;border-radius:50%;border:1.5px solid var(--accent);opacity:.7;animation:pulse-ping 2s ease-out infinite}
#lp-home .pulse-meta{font-size:.72rem;color:var(--muted);font-variant-numeric:tabular-nums}
#lp-home .pulse-wave{height:58px;margin:12px 0 6px}
#lp-home .pulse-wave svg{width:100%;height:100%;display:block}
#lp-home .wave-track{fill:none;stroke:color-mix(in srgb,var(--brand) 26%,transparent);stroke-width:2}
#lp-home .wave-pulse{fill:none;stroke:var(--brand);stroke-width:2.6;stroke-linecap:round;stroke-linejoin:round;filter:drop-shadow(0 0 5px var(--brand));stroke-dasharray:16 84;stroke-dashoffset:100;animation:pulse-sweep 2.8s linear infinite}
#lp-home .pulse-rows{position:relative}
#lp-home .prow{display:flex;align-items:center;gap:10px;padding:9px 2px;border-bottom:1px solid color-mix(in srgb,var(--line) 70%,transparent);font-size:.78rem}
#lp-home .prow:last-child{border-bottom:0}
#lp-home .pdot{width:8px;height:8px;border-radius:50%;flex:none;background:var(--accent);box-shadow:0 0 7px color-mix(in srgb,var(--accent) 70%,transparent)}
#lp-home .purl{flex:1;min-width:0;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;color:var(--ink)}
#lp-home .pb{font-size:.66rem;font-weight:700;padding:3px 10px;border-radius:99px;white-space:nowrap;position:relative;display:inline-flex;align-items:center;justify-content:center;min-width:44px;color:var(--accent);background:color-mix(in srgb,var(--accent) 15%,transparent)}
#lp-home .pb .b-lost{position:absolute;inset:0;display:inline-flex;align-items:center;justify-content:center;opacity:0}
#lp-home .prow.alertrow .pdot{animation:pulse-flipdot 9s ease-in-out infinite}
#lp-home .prow.alertrow .pb{animation:pulse-flipbg 9s ease-in-out infinite}
#lp-home .prow.alertrow .pb .b-ok{animation:pulse-fadeout 9s ease-in-out infinite}
#lp-home .prow.alertrow .pb .b-lost{animation:pulse-fadein 9s ease-in-out infinite}
#lp-home .pulse-toast{position:absolute;left:14px;right:14px;bottom:14px;display:flex;align-items:center;gap:11px;padding:11px 13px;border-radius:12px;background:color-mix(in srgb,var(--danger) 13%,var(--bg));border:1px solid color-mix(in srgb,var(--danger) 42%,var(--line));box-shadow:0 16px 32px -10px color-mix(in srgb,var(--danger) 45%,transparent);opacity:0;transform:translateY(16px);animation:pulse-toast 9s ease-in-out infinite}
#lp-home .pulse-toast-dot{width:9px;height:9px;border-radius:50%;background:var(--danger);flex:none;box-shadow:0 0 8px var(--danger)}
#lp-home .pulse-toast b{display:block;color:var(--danger);font-size:.8rem;line-height:1.2}
#lp-home .pulse-toast span{display:block;color:var(--muted);font-size:.7rem}
@keyframes pulse-sweep{to{stroke-dashoffset:0}}
@keyframes pulse-ping{0%{transform:scale(.6);opacity:.8}100%{transform:scale(2.3);opacity:0}}
@keyframes pulse-flipdot{0%,40%{background:var(--accent);box-shadow:0 0 7px color-mix(in srgb,var(--accent) 70%,transparent)}48%,72%{background:var(--danger);box-shadow:0 0 10px color-mix(in srgb,var(--danger) 85%,transparent)}80%,100%{background:var(--accent);box-shadow:0 0 7px color-mix(in srgb,var(--accent) 70%,transparent)}}
@keyframes pulse-flipbg{0%,40%{color:var(--accent);background:color-mix(in srgb,var(--accent) 15%,transparent)}48%,72%{color:var(--danger);background:color-mix(in srgb,var(--danger) 15%,transparent)}80%,100%{color:var(--accent);background:color-mix(in srgb,var(--accent) 15%,transparent)}}
@keyframes pulse-fadeout{0%,41%{opacity:1}47%,74%{opacity:0}80%,100%{opacity:1}}
@keyframes pulse-fadein{0%,42%{opacity:0}48%,72%{opacity:1}78%,100%{opacity:0}}
@keyframes pulse-toast{0%,42%{opacity:0;transform:translateY(16px)}50%,68%{opacity:1;transform:translateY(0)}76%,100%{opacity:0;transform:translateY(16px)}}
/* Hero monitor: slide-up entrance on the wrapper (so it joins the headline/CTA
   stagger), then a slow, calm float once it has arrived. Timings share the hero's
   cubic-bezier so every motion feels part of one system. */
#lp-home .hero .pulse-wrap{animation:lp-rise .8s cubic-bezier(.16,1,.3,1) .3s both}
#lp-home .hero .pulse-card{animation:pulse-float 7s ease-in-out 1.1s infinite}
/* Discreet breathing on the Live indicators (not the one that flips). */
#lp-home .prow:not(.alertrow) .pdot{animation:pulse-dot 2.8s ease-in-out infinite}
#lp-home .prow:nth-child(3) .pdot{animation-delay:.7s}
#lp-home .prow:nth-child(4) .pdot{animation-delay:1.4s}
/* Hero button micro-interaction: a soft lift on hover, settle on press. */
#lp-home .hero .btn{transition:filter .18s cubic-bezier(.16,1,.3,1),transform .18s cubic-bezier(.16,1,.3,1),box-shadow .18s cubic-bezier(.16,1,.3,1)}
#lp-home .hero .btn:hover{transform:translateY(-2px)}
#lp-home .hero .btn:not(.ghost):hover{box-shadow:0 14px 30px color-mix(in srgb,var(--brand) 34%,transparent)}
#lp-home .hero .btn:active{transform:translateY(0)}
@keyframes pulse-float{0%,100%{transform:translateY(0)}50%{transform:translateY(-6px)}}
@keyframes pulse-dot{0%,100%{box-shadow:0 0 6px color-mix(in srgb,var(--accent) 50%,transparent)}50%{box-shadow:0 0 11px color-mix(in srgb,var(--accent) 95%,transparent)}}
@media(prefers-reduced-motion:reduce){
  #lp-home .wave-pulse,#lp-home .pulse-ping::after,#lp-home .prow .pdot,#lp-home .prow.alertrow .pb,#lp-home .prow.alertrow .pb .b-ok,#lp-home .prow.alertrow .pb .b-lost,#lp-home .pulse-toast,#lp-home .hero .pulse-wrap,#lp-home .hero .pulse-card{animation:none}
  #lp-home .pulse-toast{display:none}
  #lp-home .prow.alertrow .pb .b-lost{opacity:0}
  #lp-home .hero .btn:hover{transform:none}
}
</style>`;

const MAIN = `${QUIZ_STYLE}
<!-- ============ HERO ============ -->
<section class="hero" aria-labelledby="h1">
  <div class="wrap grid">
    <div>
      <h1 id="h1">Backlink monitoring with automated alerts</h1>
      <p class="lead">Track your backlinks. Detect changes. Stay in control of your SEO. LinkPulz monitors new and lost backlinks, tracks dofollow and nofollow changes, analyzes competitor backlinks, and alerts you the moment something important changes.</p>
      <ul class="check">
        <li>Track new and lost backlinks</li>
        <li>Monitor dofollow and nofollow changes</li>
        <li>Analyze competitor backlinks</li>
        <li>Get automated email, Slack and webhook alerts</li>
      </ul>
      <div class="cta-row">
        <a class="btn" href="/register">Start your free trial</a>
        <a class="btn ghost" href="#how-it-works">Explore how it works</a>
      </div>
      <p class="micro">10-day free trial · No credit card required · Set up in minutes</p>
    </div>
    <div class="pulse-wrap"><div class="pulse-card" role="img" aria-label="LinkPulz live backlink monitor: links are checked on a pulse and a lost-link alert fires when one disappears">
      <div class="pulse-top">
        <span class="pulse-live"><span class="pulse-ping"></span>Live monitor</span>
        <span class="pulse-meta">1,284 backlinks · checking</span>
      </div>
      <div class="pulse-wave" aria-hidden="true">
        <svg viewBox="0 0 320 64" preserveAspectRatio="none">
          <path class="wave-track" pathLength="100" d="M0,40 H92 l6,-4 l5,-24 l8,50 l7,-36 l6,14 H320"></path>
          <path class="wave-pulse" pathLength="100" d="M0,40 H92 l6,-4 l5,-24 l8,50 l7,-36 l6,14 H320"></path>
        </svg>
      </div>
      <div class="pulse-rows">
        <div class="prow"><span class="pdot"></span><span class="purl">forbes.com/best-seo-tools</span><span class="pb">Live</span></div>
        <div class="prow alertrow"><span class="pdot"></span><span class="purl">blog.example.com/seo-guide</span><span class="pb"><span class="b-ok">Live</span><span class="b-lost">Lost</span></span></div>
        <div class="prow"><span class="pdot"></span><span class="purl">news.site/tool-review</span><span class="pb">Live</span></div>
        <div class="prow"><span class="pdot"></span><span class="purl">partner.io/resources</span><span class="pb">Live</span></div>
      </div>
      <div class="pulse-toast" aria-hidden="true"><span class="pulse-toast-dot"></span><div><b>Backlink lost</b><span>blog.example.com · detected just now</span></div></div>
    </div></div>
  </div>
</section>

<!-- ============ QUIZ (interactive self-assessment) ============ -->
<section class="alt" id="quiz" aria-labelledby="quiz-h">
  <div class="wrap">
    <div class="center">
      <h2 id="quiz-h">How strong is your backlink monitoring strategy?</h2>
      <p class="lead">Five quick questions, one at a time. Get your monitoring profile and the next step for you.</p>
    </div>
    <div class="quiz">
      <div class="quiz-progress">
        <span class="quiz-step" aria-live="polite">Question 1 of 5</span>
        <div class="quiz-bar" aria-hidden="true"><i style="width:20%"></i></div>
      </div>
      <ol class="quiz-q">
        <li tabindex="-1">
          <p class="quiz-ttl">1. How do you currently track your backlinks?</p>
          <div class="quiz-opts">
            <label><input type="radio" name="q1" value="0"><span>I don't track them</span></label>
            <label><input type="radio" name="q1" value="1"><span>Spreadsheets or manual checks</span></label>
            <label><input type="radio" name="q1" value="2"><span>Automated monitoring software</span></label>
          </div>
        </li>
        <li tabindex="-1">
          <p class="quiz-ttl">2. How quickly do you notice a lost backlink?</p>
          <div class="quiz-opts">
            <label><input type="radio" name="q2" value="0"><span>Usually I don't notice</span></label>
            <label><input type="radio" name="q2" value="1"><span>During my next manual SEO audit</span></label>
            <label><input type="radio" name="q2" value="2"><span>Through an automated alert</span></label>
          </div>
        </li>
        <li tabindex="-1">
          <p class="quiz-ttl">3. Do you monitor dofollow and nofollow changes?</p>
          <div class="quiz-opts">
            <label><input type="radio" name="q3" value="0"><span>No</span></label>
            <label><input type="radio" name="q3" value="1"><span>Occasionally</span></label>
            <label><input type="radio" name="q3" value="2"><span>Yes, consistently</span></label>
          </div>
        </li>
        <li tabindex="-1">
          <p class="quiz-ttl">4. How do you analyze competitor backlinks?</p>
          <div class="quiz-opts">
            <label><input type="radio" name="q4" value="0"><span>I don't analyze them</span></label>
            <label><input type="radio" name="q4" value="1"><span>I research them occasionally</span></label>
            <label><input type="radio" name="q4" value="2"><span>I use a dedicated tool</span></label>
          </div>
        </li>
        <li tabindex="-1">
          <p class="quiz-ttl">5. How do you organize your backlink data?</p>
          <div class="quiz-opts">
            <label><input type="radio" name="q5" value="0"><span>No structured process</span></label>
            <label><input type="radio" name="q5" value="1"><span>Spreadsheets or CSV files</span></label>
            <label><input type="radio" name="q5" value="2"><span>A dedicated management workflow</span></label>
          </div>
        </li>
      </ol>

      <div class="quiz-nav">
        <button type="button" class="quiz-back">&larr; Back</button>
      </div>

      <div class="quiz-res">
        <p class="cap">Three monitoring profiles. Answer the five questions above to see which one fits you.</p>
        <p class="quiz-verdict" aria-live="polite"></p>
        <div class="cards">
          <div class="card" data-tier="0">
            <h3><span class="tier" style="background:var(--danger)"></span>Reactive monitoring</h3>
            <p>Important backlink changes may be going unnoticed.</p>
            <p class="next"><b>Next step:</b> start monitoring your most important backlinks and enable automated alerts.</p>
          </div>
          <div class="card" data-tier="1">
            <h3><span class="tier" style="background:var(--warn)"></span>Developing monitoring</h3>
            <p>You have some practices, but there's room to cut manual work and improve consistency.</p>
            <p class="next"><b>Next step:</b> organize your backlink data and automate change notifications.</p>
          </div>
          <div class="card" data-tier="2">
            <h3><span class="tier" style="background:var(--accent)"></span>Proactive monitoring</h3>
            <p>You have an established monitoring workflow.</p>
            <p class="next"><b>Next step:</b> explore competitor analysis and manage monitoring across larger campaigns.</p>
          </div>
        </div>
        <div class="center" style="margin-top:16px"><button type="button" class="quiz-restart">&#8635; Start over</button></div>
      </div>

      <p class="quiz-note micro" style="margin-top:18px">This assessment reflects your self-reported habits. It is not a technical audit of your backlink profile.</p>
      <div class="quiz-cta center" style="margin-top:12px"><a class="btn" href="/register">Start your monitoring setup</a></div>
    </div>
  </div>
</section>

<!-- ============ SEE WHAT'S HAPPENING ============ -->
<section aria-labelledby="see-h">
  <div class="wrap">
    <div class="center">
      <h2 id="see-h">See what's happening to your backlinks</h2>
      <p class="lead">A backlink that exists today might disappear tomorrow. A dofollow link might become nofollow. A competitor might earn links from sites you haven't explored yet. Without consistent monitoring, these changes go unnoticed.</p>
    </div>

    <div class="split" style="margin-top:36px">
      <div>
        <h3>With LinkPulz you can</h3>
        <ul class="check">
          <li><b>Track new backlinks</b>: identify newly discovered links pointing to your site.</li>
          <li><b>Detect lost backlinks</b>: find out when a previously monitored link disappears.</li>
          <li><b>Monitor link attributes</b>: keep track of dofollow and nofollow changes.</li>
          <li><b>Receive automated alerts</b>: stay informed when a monitored change is detected.</li>
          <li><b>Analyze competitor backlinks</b>: explore the referring domains behind competing sites.</li>
          <li><b>Manage data with CSV</b>: import and export your backlink data for clean records.</li>
        </ul>
        <div class="cta-row"><a class="btn" href="/register">Start monitoring your backlinks</a></div>
      </div>

      <!-- Interactive demo scenarios (illustrative, not live data) -->
      <div class="cards" style="margin-top:0;grid-template-columns:1fr">
        <div class="card broken-case" style="padding:18px 20px">
          <div class="srow" style="border-bottom:0;padding:0 0 6px"><span class="surl"><b>Scenario 1: New backlink detected</b></span><span class="sbadge live">New</span></div>
          <p class="micro" style="margin:0">A new link to your site is discovered. Review the source, target page and attributes, then decide whether to protect it.</p>
        </div>
        <div class="card broken-case" style="padding:18px 20px">
          <div class="srow" style="border-bottom:0;padding:0 0 6px"><span class="surl"><b>Scenario 2: Backlink lost</b></span><span class="sbadge lost">Lost</span></div>
          <p class="micro" style="margin:0">A monitored link is no longer detected at its expected location. Investigate the change and decide whether it can be recovered.</p>
        </div>
        <div class="card broken-case" style="padding:18px 20px">
          <div class="srow" style="border-bottom:0;padding:0 0 6px"><span class="surl"><b>Scenario 3: Dofollow → nofollow</b></span><span class="sbadge changed">Changed</span></div>
          <p class="micro" style="margin:0">A monitored link changes its attributes. Review the updated link and decide whether further action is needed.</p>
        </div>
        <p class="micro" style="margin:2px 0 0">Example events are illustrative. Actual detection depends on the monitoring methodology.</p>
      </div>
    </div>
  </div>
</section>

<!-- ============ VERIFY BACKLINKS (try it) ============ -->
<section id="verify-backlinks" aria-labelledby="verify-h">
  <div class="wrap center">
    <h2 id="verify-h">Experience backlink monitoring in action</h2>
    <p class="lead">Check if a backlink exists, whether it's live, and how it's set up. Paste the page that links to you and your URL, and our backlink verification tool does the rest.</p>

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

<!-- ============ WHAT IS BACKLINK MONITORING? (explainer) ============ -->
<section class="alt" id="what-is" aria-labelledby="what-h">
  <div class="wrap">
    <div class="center">
      <h2 id="what-h">What is backlink monitoring?</h2>
      <p class="lead">Backlink monitoring is the process of tracking the links pointing to your website over time to identify new links, lost links and changes to existing backlinks.</p>
    </div>
    <div class="prose center" style="margin-top:8px">
      <p>Unlike a one-time backlink check, ongoing monitoring helps you understand how your backlink profile changes and alerts you when relevant events are detected. For SEO professionals, it provides visibility into link-building campaigns, acquired placements, and opportunities to recover lost links.</p>
    </div>

    <div class="split" style="margin-top:40px">
      <div>
        <h3>How does backlink monitoring work?</h3>
        <ol>
          <li><b>Track backlinks</b>: add or identify the links you want to monitor.</li>
          <li><b>Check their status</b>: the software examines each monitored link and its attributes.</li>
          <li><b>Detect changes</b>: new links, missing backlinks or modified attributes are identified.</li>
          <li><b>Send alerts</b>: automated notifications help you stay informed.</li>
          <li><b>Take action</b>: review the change and decide whether follow-up is needed.</li>
        </ol>
        <p>The result is a more structured approach to backlink management.</p>
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
  </div>
</section>

<!-- ============ WHY IT MATTERS ============ -->
<section aria-labelledby="why-h">
  <div class="wrap">
    <div class="center">
      <h2 id="why-h">Why is backlink monitoring important for SEO?</h2>
      <p class="lead">Building backlinks is only part of a successful link-building strategy. Once links are acquired, you need to know whether they stay in place and whether their attributes change.</p>
    </div>
    <div class="cards five">
      <div class="card"><div class="ic">1</div><h3>Detect lost backlinks before they go unnoticed</h3><p>Publishers remove links, update articles, delete pages or restructure sites. Monitoring tells you when a tracked link is no longer found, so you can investigate and reach out while it's recent.</p></div>
      <div class="card"><div class="ic">2</div><h3>Protect your link-building efforts</h3><p>Outreach, digital PR and partnerships take time and budget. Monitoring verifies that the placements you earned or paid for are still live.</p></div>
      <div class="card"><div class="ic">3</div><h3>Track dofollow and nofollow changes</h3><p>A publisher can change a link's attributes after publication. Monitoring surfaces the change so you can check the backlink still meets your expectations.</p></div>
      <div class="card"><div class="ic">4</div><h3>Reduce manual backlink checks</h3><p>Checking links one by one gets harder as your portfolio grows. Automation brings detected changes into a single workflow instead of repeated manual visits.</p></div>
      <div class="card"><div class="ic">5</div><h3>Understand competitor backlink activity</h3><p>Competitors' backlinks reveal outreach opportunities and relevant publications. Competitor analysis helps you explore those signals to inform your strategy.</p></div>
    </div>
    <div class="center" style="margin-top:36px"><a class="btn" href="/register">Start your free trial</a></div>
  </div>
</section>

<!-- ============ EVERYTHING YOU NEED (feature deep-dives) ============ -->
<section class="alt" id="features" aria-labelledby="feat-h">
  <div class="wrap">
    <div class="center">
      <h2 id="feat-h">Everything you need in a backlink monitoring tool</h2>
      <p class="lead">A link isn't just "there" or "gone". LinkPulz tracks every way a link can quietly stop working, and the opportunities around it.</p>
    </div>
    <div class="cards">
      <div class="card"><div class="ic">+</div><h3>Monitor new backlinks</h3><p>Keep track of newly identified links and watch how your backlink portfolio evolves.</p></div>
      <div class="card"><div class="ic">✕</div><h3>Detect lost backlinks</h3><p>Know when a monitored link disappears so you can investigate and decide whether recovery is possible.</p></div>
      <div class="card"><div class="ic">404</div><h3>Find broken backlinks</h3><p>Spot linking pages that return a 404 or 5xx, and links that now redirect elsewhere, on both source and destination.</p></div>
      <div class="card"><div class="ic">♥</div><h3>Monitor backlink health</h3><p>Track status, attributes and indexation for every link in one view.</p></div>
      <div class="card"><div class="ic">rel</div><h3>Dofollow &amp; nofollow</h3><p>Dofollow switches to nofollow, sponsored or UGC, flagged on every check.</p></div>
      <div class="card"><div class="ic">vs</div><h3>Competitor backlinks</h3><p>Explore the referring domains behind competing sites and surface gap opportunities.</p></div>
      <div class="card"><div class="ic">Aa</div><h3>Anchor text changes</h3><p>Your agreed anchor is edited or replaced, detected and logged.</p></div>
      <div class="card"><div class="ic">⤓</div><h3>CSV import &amp; export</h3><p>Bring links in from a CSV or Excel file, and export every link with its history.</p></div>
    </div>
  </div>
</section>

<!-- ============ DETECT LOST BACKLINKS ============ -->
<section id="lost-backlinks" aria-labelledby="lost-h">
  <div class="wrap">
    <div class="center">
      <h2 id="lost-h">Detect lost backlinks before they cost you rankings</h2>
      <p class="lead">You had a backlink, and now it's gone. LinkPulz tells you which links disappeared, when, and the likely cause, so you can win them back.</p>
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
        <p style="margin-top:16px">Add your links to LinkPulz. We re-check each linking page on a schedule and flag any link that is no longer there, with the likely cause. Then contact the site owner while the change is recent. <a href="/lost-backlinks">Read our guide on regaining lost backlinks</a>.</p>
      </div>
      <div class="lost-demo" role="img" aria-label="Example: a backlink goes from live yesterday to lost today, with details to win it back">
        <p class="micro verify-label">Example</p>
        <div class="srow"><span class="surl">example.com/best-seo-tools</span><span class="sbadge lost">Lost</span></div>
        <div class="stiles" style="margin-top:10px"><div class="stile"><b>3 days</b><small>Since lost</small></div><div class="stile"><b>$120</b><small>Paid placement</small></div><div class="stile"><b>example.com</b><small>Referring site</small></div></div>
        <div class="sdiff" style="margin-top:12px"><b>What changed</b> <del>dofollow · anchor “backlink monitoring”</del> → <ins>link removed</ins></div>
      </div>
    </div>
    <div class="center" style="margin-top:36px"><a class="btn" href="/register">Get alerted when a backlink disappears</a></div>
  </div>
</section>

<!-- ============ BROKEN BACKLINKS ============ -->
<section class="alt" id="broken-backlinks" aria-labelledby="broken-h">
  <div class="wrap">
    <div class="center">
      <h2 id="broken-h">Find broken backlinks before they waste your link equity</h2>
      <p class="lead">A broken backlink isn't always a lost one. The link can still be there while the page it points to (or the page it sits on) returns an error. LinkPulz catches both.</p>
    </div>
    <div class="cards">
      <div class="card broken-case">
        <h3>Case 1: your link works, but the destination is broken</h3>
        <p>Another site still links to you, but the URL it points to no longer exists.</p>
        <div class="flow" role="img" aria-label="Example: a backlink points to example.com/my-old-page, which returns a 404 error">
          <span class="url">example.com/my-old-page</span>
          <span class="arrow" aria-hidden="true">↓</span>
          <span class="status lost">● 404</span>
        </div>
        <p class="micro">Typical causes: a deleted page, a changed URL, a missing redirect after a migration.</p>
      </div>
      <div class="card broken-case">
        <h3>Case 2: the page holding your link is broken</h3>
        <p>Your backlink is on a page that now returns an error, so the link is no longer reachable.</p>
        <div class="flow" role="img" aria-label="Example: the page that contains your backlink returns a 404 or 5xx error">
          <span class="url">othersite.com/article-with-your-link</span>
          <span class="arrow" aria-hidden="true">↓</span>
          <span class="status lost">● 404 / 5xx</span>
        </div>
        <p class="micro">Typical causes: the publisher deleted the article, a server error, or the site went offline.</p>
      </div>
    </div>
    <div class="center" style="margin-top:36px"><a class="btn" href="/register">Find broken backlinks automatically</a>
      <p class="micro" style="margin-top:12px">Learn more: <a href="/broken-backlinks">how to find and fix broken backlinks</a>.</p>
    </div>
  </div>
</section>

<!-- ============ BACKLINK HEALTH CHECK ============ -->
<section id="backlink-health-check" aria-labelledby="health-h">
  <div class="wrap">
    <div class="center">
      <h2 id="health-h">See the health of your entire backlink profile</h2>
      <p class="lead">It's not just "does this link exist?" Know how healthy your whole portfolio is, and which links need attention first.</p>
    </div>
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
    <div class="center" style="margin-top:28px"><a class="btn" href="/register">Check your backlink health</a>
      <p class="micro" style="margin-top:12px">Learn more: <a href="/backlink-health-check">how to run a backlink health check</a>.</p>
    </div>
  </div>
</section>

<!-- ============ AUTOMATED ALERTS ============ -->
<section class="alt" aria-labelledby="alerts-h">
  <div class="wrap split">
    <div>
      <h2 id="alerts-h">Automated alerts that tell you what changed</h2>
      <p>Backlink changes don't happen when you're reading your SEO reports. Automated alerts keep you informed the moment a monitored event is detected, and every alert is specific: the source URL, your target URL, what changed (before and after), and the date detected. Route alerts by client or campaign, or get a weekly digest instead of real-time pings.</p>
      <ul class="check">
        <li>Email, Slack and webhook alerts</li>
        <li>Full change history for every link</li>
        <li>CSV export of every link and its history</li>
      </ul>
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

<!-- ============ COMPETITOR BACKLINKS ============ -->
<section aria-labelledby="comp-h">
  <div class="wrap split rev">
    <div class="shot" role="img" aria-label="Competitor backlink gap: domains linking to a competitor but not to you">
      <div class="shot-bar"><span class="shot-dots"><i></i><i></i><i></i></span><span class="shot-ttl">Competitor gap · Acme</span></div>
      <div class="shot-body">
        <span class="ex">Example data</span>
        <div class="srow smuted" style="font-size:.64rem;text-transform:uppercase;letter-spacing:.04em"><span class="surl">Referring domain</span><span>Links to</span><span>You</span></div>
        <div class="srow"><span class="surl">editor.news-site.com</span><span class="smuted">competitor.com</span><span class="sbadge lost">Gap</span></div>
        <div class="srow"><span class="surl">roundup.blog</span><span class="smuted">competitor.com</span><span class="sbadge lost">Gap</span></div>
        <div class="srow"><span class="surl">industry-review.io</span><span class="smuted">competitor.com</span><span class="sbadge lost">Gap</span></div>
        <div class="stags"><span class="stag">37 gap domains</span><span class="stag">→ Turn into outreach prospects</span></div>
      </div>
    </div>
    <div>
      <h2 id="comp-h">Analyze competitor backlinks</h2>
      <p>Your competitors' backlinks reveal outreach opportunities, relevant publications and link-building patterns. LinkPulz competitor gap analysis surfaces the referring domains that link to competing sites but not yet to yours: the clearest shortlist for your next campaign.</p>
      <ul class="check">
        <li>Explore competitor backlink profiles</li>
        <li>Identify domains linking to rivals but not you</li>
        <li>Turn gap domains into outreach prospects</li>
        <li>Support competitor-focused SEO research</li>
      </ul>
    </div>
  </div>
</section>

<!-- ============ CSV / DATA ============ -->
<section class="alt" aria-labelledby="csv-h">
  <div class="wrap split">
    <div>
      <h2 id="csv-h">Organize your backlink data with CSV</h2>
      <p>Keep backlink information easy to manage. Import links from a CSV or Excel file to get started in minutes, and export every monitored link (with its full change history) whenever you need clean records for a campaign, a client report or your own archive.</p>
      <ul class="check">
        <li>Import backlinks from CSV or Excel</li>
        <li>Import referring domains from Google Search Console</li>
        <li>Export every link and its change history to CSV</li>
      </ul>
    </div>
    <div class="shot" role="img" aria-label="Importing backlinks from a file">
      <div class="shot-bar"><span class="shot-dots"><i></i><i></i><i></i></span><span class="shot-ttl">Import backlinks</span></div>
      <div class="shot-body">
        <span class="ex">Example</span>
        <div class="srow"><span aria-hidden="true">📄</span><span class="surl">backlinks-q3.xlsx</span><span class="sbadge live">48 rows</span></div>
        <div class="stiles"><div class="stile"><b>46</b><small>Imported</small></div><div class="stile"><b>2</b><small>Duplicates</small></div><div class="stile"><b>0</b><small>Errors</small></div></div>
        <div class="stags"><span class="stag">CSV · Excel · GSC</span><span class="stag">Tag: Q3 outreach</span><span class="stag">Client: Acme</span></div>
      </div>
    </div>
  </div>
</section>

<!-- ============ HOW IT WORKS (4 steps) ============ -->
<section id="how-it-works" aria-labelledby="how-h">
  <div class="wrap">
    <div class="center">
      <h2 id="how-h">How to monitor backlinks in 4 simple steps</h2>
      <p class="lead">You don't need to revisit every backlink by hand to stay informed. Here's a more efficient workflow.</p>
    </div>
    <div class="steps four">
      <div class="step">
        <h3>Add the backlinks to monitor</h3>
        <p>Paste them, upload a CSV or Excel file, or import referring domains from Google Search Console. Tag each one by campaign, client or cost.</p>
      </div>
      <div class="step">
        <h3>Configure your monitoring</h3>
        <p>Set your check schedule (daily to monthly) and alert preferences. Choose the changes you want to follow, such as lost links or attribute changes.</p>
      </div>
      <div class="step">
        <h3>Review changes and alerts</h3>
        <p>Use the dashboard to examine detected changes (new backlinks, lost links and attribute updates) as they become available.</p>
      </div>
      <div class="step">
        <h3>Investigate and take action</h3>
        <p>Not every change needs the same response. Some are temporary; others call for contacting a publisher or updating your records.</p>
      </div>
    </div>
    <div class="center" style="margin-top:36px"><a class="btn" href="/register">Start monitoring backlinks free</a></div>
  </div>
</section>

<!-- ============ CHECKER vs MONITOR ============ -->
<section aria-labelledby="cm-h">
  <div class="wrap">
    <div class="center">
      <h2 id="cm-h">Backlink monitoring vs. backlink checking</h2>
      <p class="lead">A backlink checker helps you inspect links at a particular moment. Backlink monitoring focuses on tracking selected links over time and identifying relevant changes.</p>
    </div>
    <div class="tablewrap">
      <table class="compare">
        <caption class="sr-only">Backlink checker compared with a backlink monitoring tool</caption>
        <thead>
          <tr>
            <th scope="col">Feature</th>
            <th scope="col">Backlink checker</th>
            <th scope="col" class="us">Backlink monitoring tool</th>
          </tr>
        </thead>
        <tbody>
          <tr><th scope="row">Inspect existing backlinks</th><td>Yes</td><td class="us">Yes</td></tr>
          <tr><th scope="row">Track changes over time</th><td>Not always</td><td class="us">Core purpose</td></tr>
          <tr><th scope="row">Detect lost backlinks</th><td>Depends on tool</td><td class="us">Yes</td></tr>
          <tr><th scope="row">Monitor link attributes</th><td>Depends on tool</td><td class="us">Yes</td></tr>
          <tr><th scope="row">Automated change alerts</th><td>Not always</td><td class="us">Yes</td></tr>
          <tr><th scope="row">Ongoing backlink management</th><td>Limited</td><td class="us">Core use case</td></tr>
        </tbody>
      </table>
    </div>
    <p class="micro center" style="margin-top:14px">If you only need to inspect a link once, a checker may be enough. For ongoing visibility into changes, dedicated monitoring software is more appropriate.</p>
  </div>
</section>

<!-- ============ COMPETITOR TOOLS COMPARISON ============ -->
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
            <td class="us">Yes: email, Slack or webhook</td>
            <td>Email alerts for new and lost links</td>
            <td>Alerts for removed or broken backlinks</td>
            <td>No alerts, report only</td>
            <td>Yes, in tools such as SE Ranking and Ahrefs</td>
          </tr>
          <tr>
            <th scope="row">Broken link detection</th>
            <td class="us">Yes: source and destination</td>
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
    <div class="center" style="margin-top:24px"><a class="btn" href="/register">Try LinkPulz free</a></div>
  </div>
</section>

<!-- ============ WHO SHOULD USE ============ -->
<section aria-labelledby="use-h">
  <div class="wrap">
    <div class="center"><h2 id="use-h">Who should use backlink monitoring software?</h2></div>
    <div class="cards five">
      <div class="card"><h3>SEO professionals</h3><p>Keep visibility into important backlinks, review attribute changes and investigate lost links as part of your workflow.</p></div>
      <div class="card"><h3>SEO agencies</h3><p>Organize monitoring across client campaigns and support reporting with structured backlink data.</p></div>
      <div class="card"><h3>Link-building teams</h3><p>Track acquired placements, catch lost links and stay aware of changes affecting outreach campaigns.</p></div>
      <div class="card"><h3>Digital PR professionals</h3><p>Monitor links from media outreach and editorial coverage to see whether key placements remain live.</p></div>
      <div class="card"><h3>In-house marketing teams</h3><p>Keep oversight of backlink activity without relying entirely on manual checks or periodic audits.</p></div>
    </div>
  </div>
</section>

<!-- ============ DATA, ACCURACY & TRANSPARENCY ============ -->
<section class="alt" id="transparency" aria-labelledby="trust-h">
  <div class="wrap">
    <div class="center">
      <h2 id="trust-h">How our monitoring works: data, accuracy &amp; transparency</h2>
      <p class="lead">Reliable monitoring starts with understanding what your data actually means. A detected change should be interpreted in context. Monitoring should support investigation, not replace it.</p>
    </div>
    <div class="faq">
      <details>
        <summary>What does a lost backlink alert mean?</summary>
        <p>It means a previously monitored link is no longer detected according to the verification process. It does not automatically mean the link was intentionally removed, or that your rankings will decline. The next step is to review the source page and determine what changed: it may have been edited, temporarily inaccessible, or genuinely removed.</p>
      </details>
      <details>
        <summary>How often are backlinks checked?</summary>
        <p>On a schedule you set (daily, weekly, bi-weekly or monthly), available on every plan. Every check is recorded in the link's history, so you can see exactly when a change was first observed.</p>
      </details>
      <details>
        <summary>How are link changes detected?</summary>
        <p>LinkPulz re-fetches each linking page on its schedule and compares it with the last recorded state: the presence of your link, the HTTP status of the page and its destination, the rel attributes (dofollow, nofollow, sponsored, UGC), the anchor text and the indexation signals. When any of these differ, the change is logged and an alert is raised.</p>
      </details>
      <details>
        <summary>What are the limitations of backlink monitoring?</summary>
        <p>Results can be affected by inaccessible pages, temporary server errors, redirects and crawl restrictions. A page that errors once may simply be down for a moment, so a single failed check is treated with caution before a link is confirmed lost. When a status is uncertain, LinkPulz shows it as such rather than overstating a removal.</p>
      </details>
    </div>
  </div>
</section>

<!-- ============ REAL WORKFLOW ============ -->
<section aria-labelledby="work-h">
  <div class="wrap split rev">
    <div class="shot" role="img" aria-label="Lost link details to help you win it back">
      <div class="shot-bar"><span class="shot-dots"><i></i><i></i><i></i></span><span class="shot-ttl">Lost link · details</span></div>
      <div class="shot-body">
        <span class="ex">Example</span>
        <div class="srow"><span class="surl">example.com/best-seo-tools</span><span class="sbadge lost">Lost</span></div>
        <div class="sdiff"><b>What changed</b> <del>dofollow · anchor “backlink monitoring”</del> → <ins>link removed</ins></div>
        <div class="smail"><div class="smail-h"><b>Reach out</b> <span class="smuted" style="font-size:.7rem">· editor@example.com</span></div><div class="smail-b smuted">Hi, I noticed the link to acme.com in your “best SEO tools” post was removed in a recent edit. Would you be open to restoring it?</div></div>
        <div class="stags"><span class="sbtn">Draft outreach</span><span class="stag">Owner on file</span></div>
      </div>
    </div>
    <div>
      <h2 id="work-h">Built around real SEO workflows</h2>
      <p>Whether you monitor a handful of important backlinks or manage ongoing campaigns, the goal is the same: understand what changed and decide what needs attention.</p>
      <p><b>Investigating a lost backlink.</b> Say your team earned a link from an industry publication, and a later check no longer detects it. You can:</p>
      <ol>
        <li>Review the detected change.</li>
        <li>Visit the source page to investigate.</li>
        <li>Determine whether the link was removed, changed or temporarily inaccessible.</li>
        <li>Contact the publisher if appropriate.</li>
        <li>Update your link-building records.</li>
      </ol>
      <p class="micro">Illustrative example, not a customer case study.</p>
    </div>
  </div>
</section>

<!-- ============ PRICING ============ -->
<section id="pricing" aria-labelledby="price-h">
  <div class="wrap center">
    <h2 id="price-h">Simple pricing, 10-day free trial</h2>
    <p class="lead">Every plan includes a 10-day free trial, no credit card, and every monitoring feature. Pay monthly, or save two months with annual billing.</p>

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
        <div class="price-block price-monthly"><div class="price">$29<small>/mo</small></div><div class="per">billed monthly</div></div>
        <div class="price-block price-annual"><div class="price">$24<small>/mo</small></div><div class="per">$290 billed yearly</div></div>
        <div class="limits"><span><b>500</b> backlinks monitored</span><span><b>3</b> projects</span><span><b>1</b> user</span></div>
        <ul class="feats">
          <li>Checks on a schedule you set</li>
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
        <div class="price-block price-monthly"><div class="price">$59<small>/mo</small></div><div class="per">billed monthly</div></div>
        <div class="price-block price-annual"><div class="price">$49<small>/mo</small></div><div class="per">$590 billed yearly</div></div>
        <div class="limits"><span><b>2,500</b> backlinks monitored</span><span><b>10</b> projects</span><span><b>3</b> users</span></div>
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
        <div class="price-block price-monthly"><div class="price">$99<small>/mo</small></div><div class="per">billed monthly</div></div>
        <div class="price-block price-annual"><div class="price">$82<small>/mo</small></div><div class="per">$990 billed yearly</div></div>
        <div class="limits"><span><b>5,000</b> backlinks monitored</span><span><b>25</b> projects</span><span><b>5</b> users</span></div>
        <ul class="feats">
          <li>Everything in Pro</li>
          <li>Priority email support</li>
          <li>Higher monitoring limits</li>
        </ul>
        <a class="btn ghost" href="/register">Start free trial</a>
      </div>
      <div class="plan">
        <h3>Agency</h3>
        <p class="pos">For agencies managing many client portfolios.</p>
        <div class="price-block price-monthly"><div class="price">$199<small>/mo</small></div><div class="per">billed monthly</div></div>
        <div class="price-block price-annual"><div class="price">$166<small>/mo</small></div><div class="per">$1,990 billed yearly</div></div>
        <div class="limits"><span><b>15,000</b> backlinks monitored</span><span><b>75</b> projects</span><span><b>15</b> users</span></div>
        <ul class="feats">
          <li>Everything in Business</li>
          <li>Dedicated priority support</li>
          <li>Built for client portfolios</li>
        </ul>
        <a class="btn ghost" href="/register">Start free trial</a>
      </div>
    </div>

    <p class="price-note">Every plan includes all monitoring features and all alert channels (email, Slack, webhook), the plans differ by how much you track and how big your team is. No credit card for the trial. Prices in USD. Going over a limit never stops monitoring or deletes data, you just can't add new links until you upgrade.</p>
    <div class="center" style="margin-top:8px"><a class="btn ghost" href="/pricing">See full pricing &amp; FAQ →</a></div>
  </div>
</section>

<!-- ============ FAQ (must match FAQPage JSON-LD) ============ -->
<section class="alt" id="faq" aria-labelledby="faq-h">
  <div class="wrap">
    <div class="center"><h2 id="faq-h">Frequently asked questions about backlink monitoring</h2></div>
    <div class="faq">
      <details><summary>What is backlink monitoring?</summary><p>Backlink monitoring is the ongoing process of tracking the links pointing to your site to identify new backlinks, lost links and changes to existing links. Unlike a one-time check, it keeps a history and alerts you when a relevant change is detected, so you can investigate it.</p></details>
      <details><summary>How do I monitor my backlinks?</summary><p>Add the links or website you want to track to LinkPulz by pasting them, uploading a CSV or importing referring domains from Google Search Console. Set your check schedule and alert preferences, then review detected changes from the dashboard. Automated alerts keep you informed without checking each link by hand.</p></details>
      <details><summary>How often does LinkPulz check my backlinks?</summary><p>On a schedule you set: daily, weekly, bi-weekly or monthly, on every plan. Every check is logged in the link's history.</p></details>
      <details><summary>Can I receive alerts when a backlink disappears?</summary><p>Yes. LinkPulz sends automated alerts by email, Slack or webhook when a monitored link is lost, redirected, nofollowed, deindexed or has its anchor changed. Each alert shows the source URL, your target URL, what changed and when it was detected.</p></details>
      <details><summary>How can I detect lost backlinks?</summary><p>LinkPulz compares the current observed state of each linking page with what it recorded before. When a previously monitored link is no longer found, it flags the change with the likely cause so you can investigate and decide whether to win it back.</p></details>
      <details><summary>Can LinkPulz detect when a link changes from dofollow to nofollow?</summary><p>Yes. We track rel attributes (nofollow, sponsored, UGC) on every check and alert you when they change.</p></details>
      <details><summary>Can I analyze competitor backlinks?</summary><p>Yes. LinkPulz includes competitor backlink gap analysis, which surfaces referring domains that link to competing sites but not yet to yours, so you can turn them into outreach prospects.</p></details>
      <details><summary>What is the difference between a backlink checker and a backlink monitor?</summary><p>A backlink checker gives you a one-time snapshot of links at a given moment. A backlink monitor tracks the same links over time, keeps their history and alerts you when something changes.</p></details>
      <details><summary>Does losing a backlink hurt SEO?</summary><p>Losing a backlink does not automatically cause a ranking decline. Its impact depends on the link's relevance, the source page and your broader backlink profile. Monitoring helps you spot the change and decide whether it needs follow-up.</p></details>
      <details><summary>Is backlink monitoring free?</summary><p>LinkPulz offers a 10-day free trial, with no credit card required, so you can evaluate its monitoring before choosing a paid plan. Paid plans start at $29/mo based on the number of backlinks monitored. See the pricing page for current details.</p></details>
      <details><summary>Can I manage backlinks using CSV files?</summary><p>Yes. You can import backlinks from a CSV or Excel file and export every monitored link with its full change history to CSV at any time.</p></details>
    </div>
  </div>
</section>

<!-- ============ FINAL CTA ============ -->
<section class="final" aria-labelledby="final-h">
  <div class="wrap">
    <h2 id="final-h">Stop guessing what happens to your backlinks</h2>
    <p class="lead">Monitor new and lost backlinks, track attribute changes, analyze competitor backlinks and stay informed with automated alerts. Spend less time checking links and more time making informed SEO decisions.</p>
    <div class="cta-row" style="justify-content:center"><a class="btn" href="/register">Start your free trial</a></div>
    <p class="micro" style="margin-top:14px;color:#8b98b4">Backlink monitoring helps you detect and investigate changes. It does not guarantee backlink retention, recovery, or improvements in search rankings.</p>
  </div>
</section>
`;

export const HOMEPAGE_HTML = marketingPage(JSONLD, MAIN);
