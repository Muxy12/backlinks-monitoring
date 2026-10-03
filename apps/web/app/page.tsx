import type { Metadata } from "next";
import { HOMEPAGE_HTML } from "./homepage-html";

// Marketing homepage (§48). Content from the approved SEO redesign, kept verbatim;
// the visual charte is remapped to the LinkPulz dark/purple palette inside
// homepage-html.ts. Head SEO (title/description/OG) lives here because <meta>/
// <title> inside the injected body are ignored — the JSON-LD graph itself stays
// inline in the body markup. Rendered by a Server Component (zero client JS).
export const metadata: Metadata = {
  title: { absolute: "Backlink Monitoring Tool: Lost Link Alerts | LinkPulz" },
  description:
    "Monitor every backlink you earn or buy. Get instant alerts when a link is lost, nofollowed or deindexed. Start your free trial.",
  // Canonical/OG URLs resolve against metadataBase (SITE_URL) so they stay correct
  // per environment — not hardcoded to a domain that isn't live yet.
  alternates: { canonical: "/" },
  robots: { index: true, follow: true, "max-image-preview": "large" },
  openGraph: {
    type: "website",
    title: "Backlink Monitoring Tool: Lost Link Alerts | LinkPulz",
    description:
      "Monitor every backlink you earn or buy. Get instant alerts when a link is lost, nofollowed or deindexed.",
    url: "/",
    images: ["/images/og-linkpulz.jpg"],
  },
  twitter: { card: "summary_large_image" },
};

export default function HomePage() {
  return <div dangerouslySetInnerHTML={{ __html: HOMEPAGE_HTML }} />;
}
