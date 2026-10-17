import type { Metadata } from "next";
import { HERO_SIZES, HERO_SRCSET, HOMEPAGE_HTML } from "./homepage-html";

// Public, content-only page → render statically so it's served from the edge
// cache (fast TTFB). Pairs with the static marketing CSP in middleware.ts.
export const dynamic = "force-static";

// The public marketing homepage at the root "/". (Pre-launch this served a
// waitlist "coming soon"; at launch the full homepage took its place and /home
// now permanently redirects here, so there is a single canonical homepage URL.)
export const metadata: Metadata = {
  title: { absolute: "Backlink Monitoring Tool with Automated Alerts | LinkPulz" },
  description:
    "Monitor new and lost backlinks, track dofollow and nofollow changes, analyze competitor backlinks, and get automated alerts the moment a link changes. Start your free trial.",
  alternates: { canonical: "/", languages: { en: "/", "x-default": "/" } },
  robots: { index: true, follow: true, "max-image-preview": "large" },
  openGraph: {
    siteName: "LinkPulz",
    type: "website",
    title: "Backlink Monitoring Tool with Automated Alerts | LinkPulz",
    description:
      "Monitor new and lost backlinks, track dofollow and nofollow changes, analyze competitor backlinks, and get automated alerts when a link changes.",
    url: "/",
    images: ["https://ye8jhaekbbcviws9.public.blob.vercel-storage.com/banniere-dashboard-linkpulz.webp"],
  },
  twitter: { card: "summary_large_image" },
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
      "sameAs": ["https://www.linkedin.com/showcase/linkpulz", "https://x.com/linkpulz"]
    },
    {
      "@type": "WebSite",
      "@id": "https://www.linkpulz.com/#website",
      "url": "https://www.linkpulz.com/",
      "name": "LinkPulz",
      "inLanguage": "en",
      "publisher": { "@id": "https://www.linkpulz.com/#organization" }
    }
  ]
}`;

// Homepage self-assessment quiz behaviour. The homepage body is injected via
// dangerouslySetInnerHTML, where a <script> tag would NOT execute, so the logic
// lives here as a real rendered <script>. Public marketing pages use the static
// CSP (`script-src 'self' 'unsafe-inline'`, no nonce), so this inline script runs;
// it is a no-op on any page without the quiz. It runs as the parser reaches it
// (right after the injected body), turning the quiz into a one-question-at-a-time
// stepper before paint; the injected subtree is opaque to React's reconciler, so
// toggling classes/attributes inside it does not affect hydration. Picking an
// answer auto-advances (Back steps you back); once all five are in it scores 0/1/2
// per question into a 0–10 total, maps that to a tier, adds `.done`, and reveals
// the matching profile + verdict. Without JS every question shows as a plain form.
const QUIZ_SCRIPT = `(function(){
  var q=document.querySelector('#quiz .quiz'); if(!q) return;
  var lis=q.querySelectorAll('.quiz-q > li');
  var total=lis.length; if(!total) return;
  var bar=q.querySelector('.quiz-bar i');
  var label=q.querySelector('.quiz-step');
  var step=0, advancing=false;
  function render(){
    for(var k=0;k<lis.length;k++){ lis[k].classList.toggle('active', k===step); }
    q.setAttribute('data-step', step);
    if(bar){ bar.style.width=((step+1)/total*100)+'%'; }
    if(label){ label.textContent='Question '+(step+1)+' of '+total; }
  }
  function go(i){
    step = i<0 ? 0 : (i>total-1 ? total-1 : i);
    render();
    var li=lis[step]; if(li){ try{ li.focus({preventScroll:true}); }catch(e){ try{ li.focus(); }catch(e2){} } }
  }
  function compute(){
    var sum=0, all=true;
    for(var i=1;i<=total;i++){
      var el=q.querySelector('input[name="q'+i+'"]:checked');
      if(!el){all=false;} else {sum+=(parseInt(el.value,10)||0);}
    }
    if(!all){ return; }
    var tier = sum<=3 ? 0 : (sum<=7 ? 1 : 2);
    var names=['Reactive monitoring','Developing monitoring','Proactive monitoring'];
    var cards=q.querySelectorAll('.quiz-res .card');
    for(var j=0;j<cards.length;j++){
      cards[j].classList.toggle('matched', (parseInt(cards[j].getAttribute('data-tier'),10)||0)===tier);
    }
    var vd=q.querySelector('.quiz-verdict');
    if(vd){ vd.innerHTML='Your profile: <b>'+names[tier]+'</b> · '+sum+'/10'; }
    q.classList.add('done');
  }
  // Set up the stepper synchronously, before first paint.
  q.classList.add('stepper');
  render();
  q.addEventListener('change', function(e){
    var t=e.target; if(!(t && t.name && /^q[1-5]$/.test(t.name))) return;
    if(step < total-1){
      if(advancing) return; advancing=true;
      setTimeout(function(){ advancing=false; go(step+1); }, 240);
    } else {
      compute();
    }
  });
  var back=q.querySelector('.quiz-back');
  if(back){ back.addEventListener('click', function(){ if(step>0){ go(step-1); } }); }
  var restart=q.querySelector('.quiz-restart');
  if(restart){ restart.addEventListener('click', function(){
    q.classList.remove('done');
    var rs=q.querySelectorAll('input[type="radio"]'); for(var i=0;i<rs.length;i++){ rs[i].checked=false; }
    var cs=q.querySelectorAll('.quiz-res .card'); for(var j=0;j<cs.length;j++){ cs[j].classList.remove('matched'); }
    go(0);
  }); }
})();`;

// Homepage scroll-entrance motion. Runs as the parser reaches it (right after the
// injected body, before paint), so sections start hidden without a flash. Safety
// by construction: it bails without IntersectionObserver (nothing tagged -> nothing
// hidden), and the CSS only hides .reveal under prefers-reduced-motion:no-preference
// — so with no JS or motion reduced, content is fully visible. An IntersectionObserver
// gives each part a staggered entrance as it enters; a rAF-throttled scroll sweep
// (plus one on load) reveals anything scrolled past, so fast scrolling can never
// leave content hidden. The injected subtree is opaque to React, so this does not
// affect hydration.
const MOTION_SCRIPT = `(function(){
  var root=document.getElementById('lp-home'); if(!root) return;
  if(!('IntersectionObserver' in window)) return;
  var secs=root.querySelectorAll('section:not(.hero)');
  var targets=[];
  for(var s=0;s<secs.length;s++){
    var wrap=secs[s].querySelector('.wrap'); if(!wrap) continue;
    var parts=wrap.children;
    for(var i=0;i<parts.length;i++){
      var el=parts[i];
      el.classList.add('reveal');
      el.style.transitionDelay=(Math.min(i,3)*0.07)+'s';
      targets.push(el);
    }
  }
  if(!targets.length) return;
  var io=new IntersectionObserver(function(entries){
    for(var k=0;k<entries.length;k++){
      if(entries[k].isIntersecting){ entries[k].target.classList.add('in'); io.unobserve(entries[k].target); }
    }
  }, {rootMargin:'0px 0px -8% 0px', threshold:0.08});
  for(var t=0;t<targets.length;t++){ io.observe(targets[t]); }
  function sweep(){
    var vh=window.innerHeight, remaining=[];
    for(var t=0;t<targets.length;t++){
      var el=targets[t];
      if(el.classList.contains('in')) continue;
      if(el.getBoundingClientRect().top < vh*0.95){ el.classList.add('in'); io.unobserve(el); }
      else { remaining.push(el); }
    }
    targets=remaining;
    if(!targets.length){ window.removeEventListener('scroll', onScroll); window.removeEventListener('resize', onScroll); }
  }
  var last=0;
  function onScroll(){ var now=Date.now(); if(now-last>=80){ last=now; sweep(); } }
  window.addEventListener('scroll', onScroll, {passive:true});
  window.addEventListener('resize', onScroll, {passive:true});
  window.addEventListener('load', function(){ sweep(); setTimeout(sweep, 500); });
  sweep();
  // Ultimate safety net: whatever happens with the observer, timers and scrolling,
  // nothing may stay hidden. After a few seconds reveal anything still untagged.
  setTimeout(function(){ for(var t=0;t<targets.length;t++){ targets[t].classList.add('in'); } targets=[]; }, 4000);
})();`;

export default function HomePage() {
  return (
    <>
      {/* The hero dashboard image is the LCP element — preload the exact
          responsive variant the browser will pick, so the download starts before
          the HTML/CSS is parsed (big win on slow 4G). */}
      <link
        rel="preload"
        as="image"
        imageSrcSet={HERO_SRCSET}
        imageSizes={HERO_SIZES}
        fetchPriority="high"
      />
      <script type="application/ld+json" dangerouslySetInnerHTML={{ __html: JSONLD }} />
      <div dangerouslySetInnerHTML={{ __html: HOMEPAGE_HTML }} />
      <script dangerouslySetInnerHTML={{ __html: QUIZ_SCRIPT }} />
      <script dangerouslySetInnerHTML={{ __html: MOTION_SCRIPT }} />
    </>
  );
}
