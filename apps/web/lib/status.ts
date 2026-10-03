// Single source of truth for backlink status colour + label (docs/14 §1.1).
// blocked/unknown are grey ("could not verify"), never red.
export function statusColor(status: string): string {
  switch (status) {
    case "live":
      return "var(--live)";
    case "lost":
      return "var(--lost)";
    case "source_error":
    case "target_error":
      return "var(--broken)";
    case "suspected_lost":
    case "changed":
      return "var(--changed)";
    case "nofollow":
      return "var(--nofollow)";
    case "redirected":
      return "var(--redirected)";
    case "blocked":
    case "unknown":
      return "var(--unknown)";
    default:
      return "var(--pending)"; // pending
  }
}

export function statusLabel(status: string): string {
  return status.replace(/_/g, " ").replace(/\b\w/g, (c) => c.toUpperCase());
}
