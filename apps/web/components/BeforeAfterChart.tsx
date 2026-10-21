"use client";

// A dependency-free, themeable before/after bar pair for a backlink's Post-Link
// Impact. Two bars, the window BEFORE acquisition and the window AFTER, split
// by a dashed T0 marker. Deliberately plain: it shows an observed change, never a
// causal claim (the label says "around acquisition", the tooltip "T0 = acquired").
// Colors come from CSS tokens so it adapts to light/dark.

const W = 280;
const H = 150;
const PAD = { top: 22, bottom: 26, side: 20 };
const BAR_W = 56;

function signalColor(signal?: string): string {
  switch (signal) {
    case "positive":
      return "var(--live)";
    case "negative":
      return "var(--lost)";
    case "neutral":
      return "var(--changed)";
    default:
      return "var(--muted-2)";
  }
}

export function BeforeAfterChart({
  before,
  after,
  unit = "",
  signal,
  changePercent,
}: {
  before: number;
  after: number;
  unit?: string;
  signal?: string;
  changePercent?: number | null;
}) {
  const innerH = H - PAD.top - PAD.bottom;
  const max = Math.max(1, before, after);
  const h = (v: number) => Math.max(2, (innerH * v) / max);
  const base = PAD.top + innerH;

  const mid = W / 2;
  const beforeX = mid - BAR_W - 14;
  const afterX = mid + 14;
  const afterColor = signalColor(signal);

  const fmt = (n: number) =>
    n >= 1000 ? `${(n / 1000).toFixed(n >= 10000 ? 0 : 1)}k` : `${n}`;

  const delta =
    changePercent === null || changePercent === undefined
      ? after > 0 && before === 0
        ? "new"
        : null
      : `${changePercent >= 0 ? "+" : ""}${changePercent.toFixed(0)}%`;

  return (
    <figure
      className="ba-chart"
      aria-label={`${unit || "metric"} before acquisition ${before}, after acquisition ${after}`}
    >
      <svg viewBox={`0 0 ${W} ${H}`} width="100%" height={H} role="img">
        {/* baseline */}
        <line x1={PAD.side} y1={base} x2={W - PAD.side} y2={base} stroke="var(--border)" strokeWidth="1" />

        {/* before bar */}
        <rect x={beforeX} y={base - h(before)} width={BAR_W} height={h(before)} rx="3" fill="var(--surface-3)" />
        <text x={beforeX + BAR_W / 2} y={base - h(before) - 6} textAnchor="middle" className="ba-val">
          {fmt(before)}
        </text>
        <text x={beforeX + BAR_W / 2} y={base + 16} textAnchor="middle" className="ba-cap">
          Before
        </text>

        {/* after bar */}
        <rect x={afterX} y={base - h(after)} width={BAR_W} height={h(after)} rx="3" fill={afterColor} opacity="0.9" />
        <text x={afterX + BAR_W / 2} y={base - h(after) - 6} textAnchor="middle" className="ba-val">
          {fmt(after)}
        </text>
        <text x={afterX + BAR_W / 2} y={base + 16} textAnchor="middle" className="ba-cap">
          After
        </text>

        {/* T0 marker */}
        <line x1={mid} y1={PAD.top - 6} x2={mid} y2={base} stroke="var(--primary)" strokeWidth="1.5" strokeDasharray="3 3" />
        <g>
          <title>T0 = date acquired</title>
          <rect x={mid - 13} y={PAD.top - 16} width="26" height="14" rx="3" fill="var(--primary)" />
          <text x={mid} y={PAD.top - 6} textAnchor="middle" className="ba-t0">
            T0
          </text>
        </g>

        {delta && (
          <text x={W - PAD.side} y={PAD.top - 6} textAnchor="end" className="ba-delta" fill={afterColor}>
            {delta}
          </text>
        )}
      </svg>
    </figure>
  );
}
