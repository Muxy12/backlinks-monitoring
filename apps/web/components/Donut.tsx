"use client";

// A dependency-free, themeable SVG donut with a center label and a legend.
// Segments are drawn as stroked arcs on a single circle (stroke-dasharray), so
// it stays crisp at any size and needs no path math. Colors come from CSS tokens.

export type DonutSegment = { label: string; value: number; color: string };

export function Donut({
  segments,
  centerValue,
  centerLabel,
  size = 132,
}: {
  segments: DonutSegment[];
  centerValue: string;
  centerLabel?: string;
  size?: number;
}) {
  const total = segments.reduce((s, x) => s + x.value, 0);
  const stroke = Math.round(size * 0.16);
  const r = (size - stroke) / 2;
  const c = 2 * Math.PI * r;
  const cx = size / 2;

  let offset = 0;
  const arcs = segments
    .filter((s) => s.value > 0)
    .map((s) => {
      const frac = total > 0 ? s.value / total : 0;
      const len = frac * c;
      const arc = { ...s, len, gap: c - len, dash: -offset };
      offset += len;
      return arc;
    });

  return (
    <div className="donut">
      <svg viewBox={`0 0 ${size} ${size}`} width={size} height={size} role="img" aria-label={centerLabel ?? "chart"}>
        <circle cx={cx} cy={cx} r={r} fill="none" stroke="var(--surface-3)" strokeWidth={stroke} />
        {total > 0 &&
          arcs.map((a, i) => (
            <circle
              key={i}
              cx={cx}
              cy={cx}
              r={r}
              fill="none"
              stroke={a.color}
              strokeWidth={stroke}
              strokeDasharray={`${a.len} ${a.gap}`}
              strokeDashoffset={a.dash}
              strokeLinecap="butt"
              transform={`rotate(-90 ${cx} ${cx})`}
            />
          ))}
        <text x={cx} y={cx - 2} textAnchor="middle" className="donut-value">
          {centerValue}
        </text>
        {centerLabel && (
          <text x={cx} y={cx + 14} textAnchor="middle" className="donut-label">
            {centerLabel}
          </text>
        )}
      </svg>
      <ul className="donut-legend">
        {segments.map((s) => (
          <li key={s.label}>
            <span className="legend-dot" style={{ background: s.color }} />
            <span className="dl-label">{s.label}</span>
            <b>{s.value}</b>
          </li>
        ))}
      </ul>
    </div>
  );
}
