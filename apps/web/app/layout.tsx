import type { Metadata } from "next";
import { headers } from "next/headers";
import { SITE_URL } from "@/lib/site";
import { CookieBanner } from "./cookie-banner";
import { ThemeToggle } from "./theme-toggle";
import "./globals.css";
import "./theme-light.css";

// Applied before first paint (no flash of the wrong theme): use the saved choice,
// else the OS preference. Only "light" is set as an attribute; dark is the CSS
// default, so an unset/failed read simply stays dark.
const THEME_SCRIPT = `(function(){try{var t=localStorage.getItem('backlio_theme');if(!t){t=window.matchMedia&&window.matchMedia('(prefers-color-scheme: light)').matches?'light':'dark';}document.documentElement.setAttribute('data-theme',t);}catch(e){}})();`;

export const metadata: Metadata = {
  // metadataBase makes every page's `alternates.canonical` (and OG URLs) resolve
  // to an absolute URL on the canonical origin.
  metadataBase: new URL(SITE_URL),
  title: {
    default: "Backlio — Backlink Monitoring for SEO Teams",
    template: "%s — Backlio",
  },
  description:
    "Monitor every backlink you build. Track lost links, nofollow changes, anchors, campaigns, costs and warranties from one dashboard.",
  alternates: { canonical: "/" },
};

export default async function RootLayout({ children }: { children: React.ReactNode }) {
  // Reading the per-request nonce (set by middleware.ts) opts every route into
  // dynamic rendering, so Next stamps that same nonce onto its inline scripts —
  // which is what makes the strict `script-src 'nonce-…'` CSP work (docs/11 S8).
  // We reuse that nonce for the no-flash theme script so it passes the CSP too.
  const nonce = (await headers()).get("x-nonce") ?? undefined;
  return (
    <html lang="en" suppressHydrationWarning>
      <head>
        <script nonce={nonce} dangerouslySetInnerHTML={{ __html: THEME_SCRIPT }} />
      </head>
      <body>
        {children}
        <CookieBanner />
        <ThemeToggle />
      </body>
    </html>
  );
}
