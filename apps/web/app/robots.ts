import type { MetadataRoute } from "next";
import { SITE_URL } from "@/lib/site";

// robots.txt: let crawlers index the public marketing surface, keep them out of
// the authenticated app, the auth pages and the legal pages (also noindex via
// their own metadata, belt and braces). Points at the auto-generated sitemap.
export default function robots(): MetadataRoute.Robots {
  return {
    rules: [
      {
        userAgent: "*",
        allow: "/",
        disallow: [
          "/dashboard",
          "/projects",
          "/clients",
          "/competitors",
          "/graph",
          "/import",
          "/team",
          "/admin",
          "/settings",
          "/login",
          "/register",
          "/invite",
          // The email-verification page carries a ?token= query. Match that
          // precisely, a bare "/verify" prefix would also block the
          // "/verify-backlinks" marketing page (which IS in the sitemap).
          "/verify?",
          "/reset",
          "/forgot",
          "/legal",
        ],
      },
    ],
    sitemap: `${SITE_URL}/sitemap.xml`,
    host: SITE_URL,
  };
}
