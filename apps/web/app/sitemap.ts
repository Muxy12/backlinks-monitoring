import type { MetadataRoute } from "next";
import { SITE_URL } from "@/lib/site";

// Auto-refreshing sitemap. Next regenerates it on demand and at most once an hour
// (`revalidate`), and `lastModified` is stamped at generation time — so it always
// reflects the current set of public, indexable pages without a manual rebuild.
// Legal pages are intentionally absent (they are noindex); the app itself and the
// auth pages are private/utility and excluded too.
export const revalidate = 3600;

export default function sitemap(): MetadataRoute.Sitemap {
  const now = new Date();
  const routes: { path: string; priority: number; changeFrequency: MetadataRoute.Sitemap[number]["changeFrequency"] }[] = [
    { path: "/", priority: 1.0, changeFrequency: "weekly" },
    { path: "/verify-backlinks", priority: 0.8, changeFrequency: "monthly" },
    { path: "/lost-backlinks", priority: 0.8, changeFrequency: "monthly" },
    { path: "/broken-backlinks", priority: 0.8, changeFrequency: "monthly" },
    { path: "/backlink-health-check", priority: 0.8, changeFrequency: "monthly" },
    { path: "/backlink-management", priority: 0.8, changeFrequency: "monthly" },
    { path: "/backlink-audit", priority: 0.8, changeFrequency: "monthly" },
    { path: "/backlink-profile", priority: 0.8, changeFrequency: "monthly" },
    { path: "/backlink-reporting", priority: 0.8, changeFrequency: "monthly" },
    { path: "/docs", priority: 0.8, changeFrequency: "weekly" },
    { path: "/tools/backlink-checker", priority: 0.7, changeFrequency: "monthly" },
  ];

  return routes.map((r) => ({
    url: `${SITE_URL}${r.path}`,
    lastModified: now,
    changeFrequency: r.changeFrequency,
    priority: r.priority,
  }));
}
