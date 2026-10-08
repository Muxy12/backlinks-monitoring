"use client";

import { useId } from "react";
import type { GrowthPoint } from "@/lib/api";

// A dependency-free, themeable SVG area chart for the Overview's Backlink Growth.
// Plots the daily snapshot's real series — live backlinks, referring domains and
// protected links — with a soft gradient fill and a glow on the lead line, so it
// reads premium in both light and dark. Colors come from CSS tokens.

const W = 640;
const H = 200;
const PAD = { top: 16, right: 14, bottom: 22, left: 34 };

function path(points: [number, number][]): string {
  return points.map(([x, y], i) => `${i === 0 ? "M" : "L"}${x.toFixed(1)},${y.toFixed(1)}`).join(" ");
}

export function GrowthChart({ series }: { series: GrowthPoint[] }) {
  const uid = useId().replace(/:/g, "");

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
    ...series.map((p) => Math.max(p.live, p.referring_domains, p.protected)),
  );
  const x = (i: number) => PAD.left + (innerW * i) / (n - 1);
  const y = (v: number) => PAD.top + innerH - (innerH * v) / max;

  const pts = (key: keyof GrowthPoint): [number, number][] =>
    series.map((p, i) => [x(i), y(p[key] as number)]);

  const live = pts("live");
  const rd = pts("referring_domains");
  const prot = pts("protected");
  const baseY = PAD.top + innerH;
  const areaLive = `${path(live)} L${x(n - 1).toFixed(1)},${baseY.toFixed(1)} L${x(0).toFixed(1)},${baseY.toFixed(1)} Z`;

  const first = series[0].day;
  const last = series[n - 1].day;
  const end = (p: [number, number][]) => p[p.length - 1];

  // Horizontal gridlines at 0, 50%, 100% of max.
  const grid = [0, 0.5, 1];

  return (
    <figure className="growth-chart" aria-label={`Backlink growth from ${first} to ${last}`}>
      <svg viewBox={`0 0 ${W} ${H}`} width="100%" height={H} preserveAspectRatio="none" role="img">
        <defs>
          <linearGradient id={`fill-${uid}`} x1="0" y1="0" x2="0" y2="1">
            <stop offset="0%" stopColor="var(--primary)" stopOpacity="0.28" />
            <stop offset="100%" stopColor="var(--primary)" stopOpacity="0" />
          </linearGradient>
          <filter id={`glow-${uid}`} x="-20%" y="-40%" width="140%" height="180%">
            <feGaussianBlur stdDeviation="3" result="b" />
            <feMerge>
              <feMergeNode in="b" />
              <feMergeNode in="SourceGraphic" />
            </feMerge>
          </filter>
        </defs>

        {grid.map((g) => {
          const gy = PAD.top + innerH - innerH * g;
          return (
            <g key={g}>
              <line
                x1={PAD.left}
                y1={gy}
                x2={W - PAD.right}
                y2={gy}
                stroke="var(--border)"
                strokeWidth="1"
                strokeDasharray={g === 0 ? "0" : "3 4"}
              />
              <text x={PAD.left - 7} y={gy + 3.5} textAnchor="end" className="chart-axis">
                {Math.round(max * g)}
              </text>
            </g>
          );
        })}

        {/* area under the lead series */}
        <path d={areaLive} fill={`url(#fill-${uid})`} />

        {/* lines */}
        <path
          d={path(live)}
          fill="none"
          stroke="var(--primary)"
          strokeWidth="2.4"
          strokeLinejoin="round"
          strokeLinecap="round"
          filter={`url(#glow-${uid})`}
        />
        <path d={path(rd)} fill="none" stroke="var(--violet)" strokeWidth="2" strokeLinejoin="round" strokeLinecap="round" />
        <path
          d={path(prot)}
          fill="none"
          stroke="var(--cyan)"
          strokeWidth="1.8"
          strokeLinejoin="round"
          strokeLinecap="round"
          strokeDasharray="4 4"
          opacity="0.9"
        />

        {/* end-point dots */}
        {([["var(--primary)", end(live)], ["var(--violet)", end(rd)], ["var(--cyan)", end(prot)]] as const).map(
          ([c, [ex, ey]], i) => (
            <g key={i}>
              <circle cx={ex} cy={ey} r="4.5" fill={c} opacity="0.2" />
              <circle cx={ex} cy={ey} r="2.6" fill={c} stroke="var(--surface)" strokeWidth="1.5" />
            </g>
          ),
        )}
      </svg>
      <figcaption className="chart-legend">
        <span className="legend-item"><span className="legend-dot" style={{ background: "var(--primary)" }} /> Live backlinks</span>
        <span className="legend-item"><span className="legend-dot" style={{ background: "var(--violet)" }} /> Referring domains</span>
        <span className="legend-item"><span className="legend-dot" style={{ background: "var(--cyan)" }} /> Protected</span>
      </figcaption>
    </figure>
  );
}
