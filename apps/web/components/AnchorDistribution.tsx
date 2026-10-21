"use client";

import type { AnchorsView, AnchorCategory } from "@/lib/api";

// Anchor text distribution, a dependency-free view of a project's anchor mix.
// Reuses the dashboard `.dist` / `.dist-bar` / `.legend` primitives; adds a
// repeated-anchors list and an honest over-concentration observation. No score,
// no verdict, see the API note.

// One token per category, so the bar and chips read the same in light/dark.
const CAT_COLOR: Record<AnchorCategory["slug"], string> = {
  branded: "var(--primary)",
  keyword: "var(--violet)",
  naked_url: "var(--live)",
  generic: "var(--pending)",
  empty: "var(--unknown)",
};

function fmtPct(p: number): string {
  // Whole number unless it would read as 0% for a non-zero slice.
  return p > 0 && p < 1 ? "<1%" : `${Math.round(p)}%`;
}

export function AnchorDistribution({ data }: { data: AnchorsView }) {
  if (data.total === 0) {
    return (
      <section className="dist anchors-dist">
        <div className="dist-head">
          <h2>Anchor text distribution</h2>
        </div>
        <p className="muted" style={{ fontSize: 13 }}>
          No anchors yet. As your links are checked, the anchor text we see on each one
          builds this breakdown.
        </p>
      </section>
    );
  }

  const shown = data.categories.filter((c) => c.count > 0);

  return (
    <section className="dist anchors-dist">
      <div className="dist-head">
        <h2>Anchor text distribution</h2>
        <span className="dist-total">
          {data.total} link{data.total > 1 ? "s" : ""} · {data.distinct} distinct anchor
          {data.distinct > 1 ? "s" : ""}
        </span>
      </div>

      <div className="dist-bar" role="img" aria-label="Anchor category mix">
        {shown.map((c) => (
          <div
            key={c.slug}
            className="dist-seg"
            style={{ flexGrow: c.count, background: CAT_COLOR[c.slug] }}
            title={`${c.label}: ${c.count} (${fmtPct(c.percent)})`}
          />
        ))}
      </div>

      <div className="legend">
        {data.categories.map((c) => (
          <span key={c.slug} className="legend-item" style={{ color: CAT_COLOR[c.slug] }}>
            <span className="legend-dot" />
            <span style={{ color: "var(--muted)" }}>{c.label}</span>
            <b>{c.count}</b>
            <span style={{ color: "var(--muted)" }}>({fmtPct(c.percent)})</span>
          </span>
        ))}
      </div>

      {data.over_concentration && (
        <p className="anchors-note" role="note">
          <b>{fmtPct(data.over_concentration.percent)}</b> of your anchors are the single
          phrase <q>{data.over_concentration.anchor}</q>. Natural link profiles usually vary
          more, worth a look, per{" "}
          <a
            href="https://developers.google.com/search/docs/essentials/spam-policies#link-spam"
            target="_blank"
            rel="noopener noreferrer"
          >
            Google&apos;s link guidance
          </a>
          . An observation, not a verdict.
        </p>
      )}

      {data.top_anchors.length > 0 && (
        <div className="anchors-top">
          <div className="anchors-top-head">Most-repeated anchors</div>
          <ul className="anchors-list">
            {data.top_anchors.map((a) => (
              <li key={`${a.slug}:${a.anchor}`} className="anchors-row">
                <span
                  className="anchors-tag"
                  style={{ color: CAT_COLOR[a.slug] }}
                  title={a.label}
                >
                  <span className="legend-dot" />
                </span>
                <span className="anchors-phrase" title={a.anchor}>
                  {a.anchor}
                </span>
                <span className="anchors-count">
                  {a.count} <span className="muted">({fmtPct(a.percent)})</span>
                </span>
              </li>
            ))}
          </ul>
        </div>
      )}

      <p className="anchors-caveat">{data.note}</p>
    </section>
  );
}
