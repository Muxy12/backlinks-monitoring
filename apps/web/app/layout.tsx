import type { Metadata } from "next";
import { headers } from "next/headers";
import "./globals.css";

export const metadata: Metadata = {
  title: "Backlio — Backlink Monitoring for SEO Teams",
  description:
    "Monitor every backlink you build. Track lost links, nofollow changes, anchors, campaigns, costs and warranties from one dashboard.",
};

export default async function RootLayout({ children }: { children: React.ReactNode }) {
  // Reading the per-request nonce (set by middleware.ts) opts every route into
  // dynamic rendering, so Next stamps that same nonce onto its inline scripts —
  // which is what makes the strict `script-src 'nonce-…'` CSP work (docs/11 S8).
  await headers();
  return (
    <html lang="en">
      <body>{children}</body>
    </html>
  );
}
