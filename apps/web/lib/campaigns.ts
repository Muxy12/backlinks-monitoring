// Vocabularies + labels for link-building campaigns (BETA, migration 0036).
// Stage/source slugs are what the API stores/validates; these map them to
// display labels and colors. Keep in sync with STAGES in campaigns_routes.rs.

export const STAGES: [string, string][] = [
  ["identified", "Identified"],
  ["qualified", "Qualified"],
  ["contacted", "Contacted"],
  ["negotiating", "Negotiating"],
  ["won", "Won"],
  ["lost", "Lost"],
];

/** Ordered pipeline stages (excludes the terminal `lost`) for the board. */
export const PIPELINE_STAGES = ["identified", "qualified", "contacted", "negotiating", "won"];

const STAGE_LABELS: Record<string, string> = Object.fromEntries(STAGES);

export function stageLabel(slug?: string | null): string {
  return slug ? (STAGE_LABELS[slug] ?? slug) : "";
}

/** Badge colors per stage (bg / fg), matching the monitoring badge palette. */
export const STAGE_BADGE: Record<string, { bg: string; fg: string }> = {
  identified: { bg: "#f1f5f9", fg: "#64748b" },
  qualified: { bg: "#e5edff", fg: "#1e40af" },
  contacted: { bg: "#fef9c3", fg: "#854d0e" },
  negotiating: { bg: "#ffedd5", fg: "#9a3412" },
  won: { bg: "#dcfce7", fg: "#166534" },
  lost: { bg: "#fee2e2", fg: "#991b1b" },
};

const SOURCE_LABELS: Record<string, string> = {
  manual: "Manual",
  competitor_gap: "Competitor gap",
  discovery: "Discovery",
};
export function sourceLabel(slug?: string | null): string {
  return slug ? (SOURCE_LABELS[slug] ?? slug) : "";
}
