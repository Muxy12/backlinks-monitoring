// Presentation formatters for the Overview (and reusable elsewhere). The API
// returns money per-currency (there is no FX); these render it without ever
// summing across currencies.

/** Thousands-separated integer. */
export function num(n: number): string {
  return new Intl.NumberFormat().format(n);
}

/** A single currency amount, e.g. "€24,850" (cents only when non-zero). */
export function money(amount: string | number, currency?: string | null): string {
  const n = typeof amount === "string" ? parseFloat(amount) : amount;
  if (!isFinite(n)) return "—";
  const cur = (currency || "").toUpperCase();
  try {
    return new Intl.NumberFormat(undefined, {
      style: "currency",
      currency: cur || "USD",
      maximumFractionDigits: n % 1 === 0 ? 0 : 2,
    }).format(n);
  } catch {
    return `${new Intl.NumberFormat().format(Math.round(n))} ${cur}`.trim();
  }
}

/** A per-currency money list (no FX): the dominant amount + "+N more". */
export function moneyList(rows: { currency: string; amount: string }[] | null | undefined): string {
  if (!rows || rows.length === 0) return "—";
  const sorted = [...rows].sort((a, b) => parseFloat(b.amount) - parseFloat(a.amount));
  const head = money(sorted[0].amount, sorted[0].currency);
  return sorted.length === 1 ? head : `${head} +${sorted.length - 1} more`;
}

/** Percent, or "—" when null. */
export function pct(x: number | null | undefined, digits = 0): string {
  return x == null ? "—" : `${x.toFixed(digits)}%`;
}

/** Compact relative time, e.g. "8m ago", "3d ago". */
export function timeAgo(iso: string): string {
  const t = new Date(iso).getTime();
  if (isNaN(t)) return "";
  const s = Math.max(0, Math.floor((Date.now() - t) / 1000));
  if (s < 60) return "just now";
  const m = Math.floor(s / 60);
  if (m < 60) return `${m}m ago`;
  const h = Math.floor(m / 60);
  if (h < 24) return `${h}h ago`;
  const d = Math.floor(h / 24);
  if (d < 30) return `${d}d ago`;
  const mo = Math.floor(d / 30);
  if (mo < 12) return `${mo}mo ago`;
  return `${Math.floor(mo / 12)}y ago`;
}
