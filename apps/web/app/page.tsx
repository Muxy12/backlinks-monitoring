import type { Metadata } from "next";
import { HOMEPAGE_HTML } from "./homepage-html";

// The public marketing homepage at the root "/". (Pre-launch this served a
// waitlist "coming soon"; at launch the full homepage took its place and /home
// now permanently redirects here, so there is a single canonical homepage URL.)
export const metadata: Metadata = {
  title: { absolute: "Backlink Monitoring Tool: Lost Link Alerts | LinkPulz" },
  description:
    "Monitor every backlink you earn or buy. Get instant alerts when a link is lost, nofollowed or deindexed. Start your free trial.",
  alternates: { canonical: "/" },
  robots: { index: true, follow: true, "max-image-preview": "large" },
  openGraph: {
    type: "website",
    title: "Backlink Monitoring Tool: Lost Link Alerts | LinkPulz",
    description:
      "Monitor every backlink you earn or buy. Get instant alerts when a link is lost, nofollowed or deindexed.",
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
      "sameAs": ["https://www.linkedin.com/company/linkpulz", "https://x.com/linkpulz"]
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

export default function HomePage() {
  return (
    <>
      <script type="application/ld+json" dangerouslySetInnerHTML={{ __html: JSONLD }} />
      <div dangerouslySetInnerHTML={{ __html: HOMEPAGE_HTML }} />
    </>
  );
}
