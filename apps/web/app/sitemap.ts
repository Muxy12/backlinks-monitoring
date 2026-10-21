import type { MetadataRoute } from "next";
import { SITE_URL } from "@/lib/site";
import { GLOSSARY } from "./glossary/glossary-data";
import { PERSONAS } from "./backlink-monitoring-for/persona-data";
import { ALTERNATIVES } from "./alternatives/alternatives-data";

// Auto-refreshing sitemap. Next regenerates it on demand and at most once an hour
// (`revalidate`), and `lastModified` is stamped at generation time, so it always
// reflects the current set of public, indexable pages without a manual rebuild.
// Legal pages are intentionally absent (they are noindex); the app itself and the
// auth pages are private/utility and excluded too.
export const revalidate = 3600;

export default function sitemap(): MetadataRoute.Sitemap {
  const now = new Date();
  const routes: { path: string; priority: number; changeFrequency: MetadataRoute.Sitemap[number]["changeFrequency"] }[] = [
    { path: "/", priority: 1.0, changeFrequency: "weekly" },
    { path: "/pricing", priority: 0.9, changeFrequency: "monthly" },
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
    { path: "/glossary", priority: 0.7, changeFrequency: "monthly" },
    ...GLOSSARY.map((t) => ({
      path: `/glossary/${t.slug}`,
      priority: 0.6,
      changeFrequency: "monthly" as const,
    })),
    ...PERSONAS.map((p) => ({
      path: `/backlink-monitoring-for/${p.slug}`,
      priority: 0.7,
      changeFrequency: "monthly" as const,
    })),
    { path: "/alternatives", priority: 0.7, changeFrequency: "monthly" },
    ...ALTERNATIVES.map((a) => ({
      path: `/alternatives/${a.slug}`,
      priority: 0.7,
      changeFrequency: "monthly" as const,
    })),
  ];

  return routes.map((r) => ({
    url: `${SITE_URL}${r.path}`,
    lastModified: now,
    changeFrequency: r.changeFrequency,
    priority: r.priority,
  }));
}
