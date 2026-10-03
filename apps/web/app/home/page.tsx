import type { Metadata } from "next";
import { HOMEPAGE_HTML } from "../homepage-html";

// The full marketing homepage, relocated to /home for the pre-launch period —
// the root "/" is the "coming soon" waitlist. Same content and JSON-LD; the
// canonical points here. The shared marketing header/footer link to /home too.
export const metadata: Metadata = {
  title: { absolute: "Backlink Monitoring Tool: Lost Link Alerts | LinkPulz" },
  description:
    "Monitor every backlink you earn or buy. Get instant alerts when a link is lost, nofollowed or deindexed. Start your free trial.",
  alternates: { canonical: "/home" },
  robots: { index: true, follow: true, "max-image-preview": "large" },
  openGraph: {
    type: "website",
    title: "Backlink Monitoring Tool: Lost Link Alerts | LinkPulz",
    description:
      "Monitor every backlink you earn or buy. Get instant alerts when a link is lost, nofollowed or deindexed.",
    url: "/home",
    images: ["https://ye8jhaekbbcviws9.public.blob.vercel-storage.com/banniere-dashboard-linkpulz.webp"],
  },
  twitter: { card: "summary_large_image" },
};

export default function MarketingHomePage() {
  return <div dangerouslySetInnerHTML={{ __html: HOMEPAGE_HTML }} />;
}
