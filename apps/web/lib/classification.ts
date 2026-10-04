// Controlled vocabularies for backlink classification (migration 0034).
// Slugs are what the API stores/validates; these map them to display labels and
// fix the option order. Keep in sync with the server lists in resource_routes.rs.

export const ACQUISITION: [string, string][] = [
  ["paid", "Paid"],
  ["exchange", "Exchange"],
  ["earned", "Earned"],
  ["outreach", "Outreach"],
  ["partnership", "Partnership"],
  ["self_created", "Self-created"],
];

export const ATTRIBUTE: [string, string][] = [
  ["follow", "Follow"],
  ["nofollow", "Nofollow"],
  ["sponsored", "Sponsored"],
  ["ugc", "UGC"],
];

export const PLACEMENT: [string, string][] = [
  ["in_content", "In Content / Contextual"],
  ["author_bio", "Author Bio"],
  ["homepage", "Homepage"],
  ["sidebar", "Sidebar"],
  ["footer", "Footer"],
  ["navigation", "Navigation"],
  ["resource_list", "Resource List"],
  ["image", "Image"],
  ["widget", "Widget"],
  ["sitewide", "Sitewide"],
];

export const FORMAT: [string, string][] = [
  ["guest_post", "Guest Post"],
  ["niche_edit", "Niche Edit"],
  ["editorial", "Editorial"],
  ["pr", "PR"],
  ["directory", "Directory"],
  ["forum", "Forum"],
];

const LABELS: Record<string, string> = Object.fromEntries([
  ...ACQUISITION,
  ...PLACEMENT,
  ...FORMAT,
  ...ATTRIBUTE,
]);

/** Display label for any classification slug (falls back to the slug). */
export function classLabel(slug?: string | null): string {
  return slug ? (LABELS[slug] ?? slug) : "";
}
