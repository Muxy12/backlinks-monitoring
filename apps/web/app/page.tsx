import type { Metadata } from "next";
import { WaitlistLanding } from "./WaitlistLanding";

// Pre-launch "coming soon" homepage with waitlist email capture. The full
// marketing homepage lives at /home during this period. Server component for
// head SEO + brand JSON-LD; the interactive landing is a client island.
export const metadata: Metadata = {
  title: { absolute: "LinkPulz: Backlink Monitoring, Launching Soon" },
  description:
    "LinkPulz monitors every backlink you earn or buy, automatic checks every 48 hours and instant alerts when a link is lost, nofollowed, redirected or broken. Join the waitlist for early access.",
  alternates: { canonical: "/" },
  robots: { index: true, follow: true, "max-image-preview": "large" },
  openGraph: {
    type: "website",
    title: "LinkPulz: Backlink Monitoring, Launching Soon",
    description:
      "Automatic backlink monitoring with instant change alerts. Join the waitlist for early access.",
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
      <WaitlistLanding />
    </>
  );
}
