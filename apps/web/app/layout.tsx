import type { Metadata } from "next";
import "./globals.css";

export const metadata: Metadata = {
  title: "Backlio — Backlink Monitoring for SEO Teams",
  description:
    "Monitor every backlink you build. Track lost links, nofollow changes, anchors, campaigns, costs and warranties from one dashboard.",
};

export default function RootLayout({ children }: { children: React.ReactNode }) {
  return (
    <html lang="en">
      <body>{children}</body>
    </html>
  );
}
