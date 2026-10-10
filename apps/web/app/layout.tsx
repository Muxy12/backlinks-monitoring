import type { Metadata } from "next";
import { headers } from "next/headers";
import { Inter } from "next/font/google";
import { SITE_URL } from "@/lib/site";
import { Analytics } from "./analytics";
import { CookieBanner } from "./cookie-banner";
import { NewsletterSignup } from "./NewsletterSignup";
import { ThemeToggle } from "./theme-toggle";
import "./tokens.css";
import "./globals.css";
import "./theme-light.css";

// Self-hosted (no external request, no CLS). Exposed as --font-inter, which
// tokens.css folds into --font-sans ahead of the system fallback stack.
const inter = Inter({
  subsets: ["latin"],
  display: "swap",
  variable: "--font-inter",
});

// Applied before first paint (no flash of the wrong theme): use the saved choice,
// else the OS preference, else light. The attribute is always set explicitly
// ("light" or "dark"); light is the design default, so a failed read stays light.
const THEME_SCRIPT = `(function(){try{var t=localStorage.getItem('backlio_theme');if(!t){t=window.matchMedia&&window.matchMedia('(prefers-color-scheme: dark)').matches?'dark':'light';}document.documentElement.setAttribute('data-theme',t);}catch(e){document.documentElement.setAttribute('data-theme','light');}})();`;

// First-touch marketing attribution (docs/20 §P8). Runs site-wide before React;
// stores utm_* + referrer + landing path in localStorage the first time a visit
// carries a signal (utm params or an external referrer). First-touch only — a
// later visit never overwrites it — and it's read once at signup.
const ATTRIBUTION_SCRIPT = `(function(){try{var K='backlio_attribution';if(localStorage.getItem(K))return;var p=new URLSearchParams(location.search);var g=function(k){var v=p.get(k);return v?v.slice(0,400):'';};var ref=document.referrer||'';var ext=false;if(ref){try{ext=new URL(ref).hostname!==location.hostname;}catch(e){}}var us=g('utm_source'),um=g('utm_medium'),uc=g('utm_campaign');if(us||um||uc||ext){localStorage.setItem(K,JSON.stringify({utm_source:us,utm_medium:um,utm_campaign:uc,utm_term:g('utm_term'),utm_content:g('utm_content'),referrer:ref.slice(0,400),landing_path:location.pathname.slice(0,400)}));}}catch(e){}})();`;

export const metadata: Metadata = {
  // metadataBase makes every page's `alternates.canonical` (and OG URLs) resolve
  // to an absolute URL on the canonical origin.
  metadataBase: new URL(SITE_URL),
  title: {
    default: "LinkPulz, Backlink Monitoring for SEO Teams",
    template: "%s | LinkPulz",
  },
  description:
    "Monitor every backlink you build. Track lost links, nofollow changes, anchors, campaigns, costs and warranties from one dashboard.",
  // No default canonical here: each page declares its own `alternates.canonical`.
  // A blanket "/" would wrongly canonicalize every page that doesn't override it.
};

export default async function RootLayout({ children }: { children: React.ReactNode }) {
  // Reading the per-request nonce (set by middleware.ts) opts every route into
  // dynamic rendering, so Next stamps that same nonce onto its inline scripts —
  // which is what makes the strict `script-src 'nonce-…'` CSP work (docs/11 S8).
  // We reuse that nonce for the no-flash theme script so it passes the CSP too.
  const nonce = (await headers()).get("x-nonce") ?? undefined;
  return (
    <html lang="en" className={inter.variable} suppressHydrationWarning>
      <head>
        {/* Blob storage serves the brand assets (logo, hero) — open the
            connection early so they aren't gated behind DNS+TLS on first paint. */}
        <link rel="preconnect" href="https://ye8jhaekbbcviws9.public.blob.vercel-storage.com" />
        <link rel="dns-prefetch" href="https://ye8jhaekbbcviws9.public.blob.vercel-storage.com" />
        <script nonce={nonce} dangerouslySetInnerHTML={{ __html: THEME_SCRIPT }} />
        <script nonce={nonce} dangerouslySetInnerHTML={{ __html: ATTRIBUTION_SCRIPT }} />
      </head>
      <body>
        {children}
        <NewsletterSignup />
        <CookieBanner />
        <ThemeToggle />
        <Analytics />
      </body>
    </html>
  );
}
