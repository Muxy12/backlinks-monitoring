import type { Metadata } from "next";
import { headers } from "next/headers";
import { Inter } from "next/font/google";
import { SITE_URL } from "@/lib/site";
import { CookieBanner } from "./cookie-banner";
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

export const metadata: Metadata = {
  // metadataBase makes every page's `alternates.canonical` (and OG URLs) resolve
  // to an absolute URL on the canonical origin.
  metadataBase: new URL(SITE_URL),
  title: {
    default: "LinkPulz — Backlink Monitoring for SEO Teams",
    template: "%s — LinkPulz",
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
    <html lang="en" className={inter.variable} suppressHydrationWarning>
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
