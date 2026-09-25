// Single source of truth for backlink status colour + label (docs/14 §1.1).
// blocked/unknown are grey ("could not verify"), never red.
export function statusColor(status: string): string {
  switch (status) {
    case "live":
      return "var(--green)";
    case "lost":
    case "source_error":
    case "target_error":
      return "var(--red)";
    case "suspected_lost":
    case "changed":
    case "nofollow":
    case "redirected":
      return "var(--yellow)";
    case "blocked":
    case "unknown":
      return "var(--muted)";
    default:
      return "var(--muted-2)"; // pending
  }
}

export function statusLabel(status: string): string {
  return status.replace(/_/g, " ").replace(/\b\w/g, (c) => c.toUpperCase());
}
