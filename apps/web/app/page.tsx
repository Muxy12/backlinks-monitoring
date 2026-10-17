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

// Interactive scoring for the homepage self-assessment quiz. The homepage body is
// injected via dangerouslySetInnerHTML, where a <script> tag would NOT execute —
// so the behaviour lives here, as a real rendered <script>. Public marketing pages
// use the static CSP (`script-src 'self' 'unsafe-inline'`, no nonce), so this
// inline script runs; it is a no-op on any page that lacks the quiz. It only ever
// mutates the DOM from a user `change` event (never before React hydrates, which
// would corrupt the injected subtree): once all five answers are in it scores
// 0/1/2 per question into a 0–10 total, maps that to a tier, adds `.done`, and
// reveals the matching profile + verdict. Without JS the quiz degrades to its
// static three-profile preview.
const QUIZ_SCRIPT = `(function(){
  function compute(){
    var q=document.querySelector('#quiz .quiz'); if(!q) return;
    var sum=0, all=true;
    for(var i=1;i<=5;i++){
      var el=q.querySelector('input[name="q'+i+'"]:checked');
      if(!el){all=false;} else {sum+=(parseInt(el.value,10)||0);}
    }
    if(!all){ q.classList.remove('done'); return; }
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
  document.addEventListener('change', function(e){
    var t=e.target;
    if(t && t.name && /^q[1-5]$/.test(t.name)){ compute(); }
  });
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
    </>
  );
}
