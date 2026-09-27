import type { Metadata } from "next";
import { headers } from "next/headers";
import { SITE_URL } from "@/lib/site";
import { CookieBanner } from "./cookie-banner";
import "./globals.css";

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
  await headers();
  return (
    <html lang="en">
      <body>
        {children}
        <CookieBanner />
      </body>
    </html>
  );
}
