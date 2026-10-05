// Vocabularies + labels for selective monitoring (migration 0035). Slugs are
// what the API stores/validates; these map them to display labels. Keep in sync
// with the server lists in resource_routes.rs.

export const MONITORING_LEVEL: [string, string][] = [
  ["protected", "Protected"],
  ["tracked", "Tracked"],
  ["inventory_only", "Inventory"],
];

export const CHECK_FREQUENCY: [string, string][] = [
  ["daily", "Daily"],
  ["weekly", "Weekly"],
  ["biweekly", "Every 2 weeks"],
  ["monthly", "Monthly"],
  ["manual", "Manual"],
];

export const COMMERCIAL_MODEL: [string, string][] = [
  ["one_time", "One-time"],
  ["rental", "Rental"],
  ["recurring", "Recurring"],
  ["exchange", "Exchange"],
  ["partnership", "Partnership"],
  ["free", "Free"],
  ["unknown", "Unknown"],
];

const REASON: Record<string, string> = {
  economic_value: "Economic value",
  paid_link: "Paid link",
  exchange: "Exchange",
  partnership: "Partnership",
  guarantee: "Guarantee",
  strategic: "Strategic",
  manual: "Manual",
  other: "Other",
};

const LABELS: Record<string, string> = {
  ...Object.fromEntries([...MONITORING_LEVEL, ...CHECK_FREQUENCY, ...COMMERCIAL_MODEL]),
  ...REASON,
};

/** Display label for any monitoring/economic slug (falls back to the slug). */
export function monitoringLabel(slug?: string | null): string {
  return slug ? (LABELS[slug] ?? slug) : "";
}

/** Badge colors per monitoring level. */
export const LEVEL_BADGE: Record<string, { bg: string; fg: string }> = {
  protected: { bg: "#dcfce7", fg: "#166534" },
  tracked: { bg: "#e5edff", fg: "#1e40af" },
  inventory_only: { bg: "#f1f5f9", fg: "#64748b" },
};
