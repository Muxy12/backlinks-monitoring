import type { Metadata } from "next";

// The checker page itself is a client component (interactive form), so its
// canonical + title live in this route layout.
export const metadata: Metadata = {
  title: "Free Backlink Checker",
  description:
    "Check whether a page links to a target URL — see rel/dofollow, anchor text and HTTP status. Free, no signup.",
  alternates: { canonical: "/tools/backlink-checker" },
};

export default function CheckerLayout({ children }: { children: React.ReactNode }) {
  return children;
}
