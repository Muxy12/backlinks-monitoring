import type { Metadata } from "next";

// Every /legal/* page is noindex: legal boilerplate should not compete in search
// or dilute the indexable surface. `robots` set here applies to all nested pages
// (child metadata may add a title but does not reset this). Also excluded from
// the sitemap and disallowed in robots.txt.
export const metadata: Metadata = {
  robots: { index: false, follow: false, nocache: true },
};

export default function LegalLayout({ children }: { children: React.ReactNode }) {
  return children;
}
