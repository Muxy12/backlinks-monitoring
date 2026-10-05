"use client";

import type { GrowthPoint } from "@/lib/api";

// A dependency-free, themeable SVG line chart for the Overview's Backlink Growth.
// Plots up to two series (referring domains + protected links) from the daily
// snapshot. Colors come from CSS tokens so it adapts to light/dark automatically.

const W = 620;
const H = 180;
const PAD = { top: 12, right: 12, bottom: 20, left: 32 };

function path(points: [number, number][]): string {
  return points.map(([x, y], i) => `${i === 0 ? "M" : "L"}${x.toFixed(1)},${y.toFixed(1)}`).join(" ");
}

export function GrowthChart({ series }: { series: GrowthPoint[] }) {
  if (!series || series.length < 2) {
    return (
      <div className="chart-empty" role="img" aria-label="Not enough history to plot yet">
        <span>Growth trend will appear here as daily snapshots accrue.</span>
      </div>
    );
  }

  const innerW = W - PAD.left - PAD.right;
  const innerH = H - PAD.top - PAD.bottom;
  const n = series.length;
  const max = Math.max(
    1,
    ...series.map((p) => Math.max(p.referring_domains, p.protected)),
  );
  const x = (i: number) => PAD.left + (innerW * i) / (n - 1);
  const y = (v: number) => PAD.top + innerH - (innerH * v) / max;

  const rd: [number, number][] = series.map((p, i) => [x(i), y(p.referring_domains)]);
  const prot: [number, number][] = series.map((p, i) => [x(i), y(p.protected)]);
  const areaRd = `${path(rd)} L${x(n - 1).toFixed(1)},${(PAD.top + innerH).toFixed(1)} L${x(0).toFixed(1)},${(PAD.top + innerH).toFixed(1)} Z`;

  const first = series[0].day;
  const last = series[n - 1].day;

  return (
    <figure className="growth-chart" aria-label={`Backlink growth from ${first} to ${last}`}>
      <svg viewBox={`0 0 ${W} ${H}`} width="100%" height={H} preserveAspectRatio="none" role="img">
        {/* gridline at the max */}
        <line
          x1={PAD.left}
          y1={PAD.top}
          x2={W - PAD.right}
          y2={PAD.top}
          stroke="var(--border)"
          strokeWidth="1"
        />
        <text x={PAD.left - 6} y={PAD.top + 4} textAnchor="end" className="chart-axis">
          {max}
        </text>
        <text x={PAD.left - 6} y={PAD.top + innerH + 4} textAnchor="end" className="chart-axis">
          0
        </text>
        <path d={areaRd} fill="var(--primary)" opacity="0.08" />
        <path d={path(rd)} fill="none" stroke="var(--primary)" strokeWidth="2" strokeLinejoin="round" />
        <path d={path(prot)} fill="none" stroke="var(--violet)" strokeWidth="2" strokeLinejoin="round" />
      </svg>
      <figcaption className="chart-legend">
        <span className="legend-dot" style={{ background: "var(--primary)" }} /> Referring domains
        <span className="legend-dot" style={{ background: "var(--violet)", marginLeft: 14 }} /> Protected
      </figcaption>
    </figure>
  );
}
