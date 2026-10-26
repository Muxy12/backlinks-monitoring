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

  // Bars scale to the largest category (fills the track) for easy length
  // comparison; the % next to each bar keeps the absolute share honest. Sorted
  // so the mix reads as a ranking, biggest first.
  const maxCount = Math.max(...data.categories.map((c) => c.count), 1);
  const bars = [...data.categories].sort((a, b) => b.count - a.count);

  // Most-repeated anchors as a donut: the 5 most representative anchors, each
  // colored by its category, plus a muted "others" rest, as fractions of all
  // anchored links. The center shows what share those 5 cover.
  const topAnchors = data.top_anchors.slice(0, 5);
  const topSum = topAnchors.reduce((s, a) => s + a.count, 0);
  const othersCount = Math.max(0, data.total - topSum);
  const top5Pct = data.total > 0 ? (topSum / data.total) * 100 : 0;
  let acc = 0;
  const ringStops = [
    ...topAnchors.map((a) => ({ color: CAT_COLOR[a.slug], value: a.count })),
    { color: "var(--surface-3)", value: othersCount },
  ]
    .filter((s) => s.value > 0)
    .map((s) => {
      const from = (acc / data.total) * 360;
      acc += s.value;
      const to = (acc / data.total) * 360;
      return `${s.color} ${from}deg ${to}deg`;
    })
    .join(", ");

  return (
    <section className="dist anchors-dist">
      <div className="dist-head">
        <h2>Anchor text distribution</h2>
        <span className="dist-total">
          {data.total} link{data.total > 1 ? "s" : ""} · {data.distinct} distinct anchor
          {data.distinct > 1 ? "s" : ""}
        </span>
      </div>

      <ul className="anchors-bars" aria-label="Anchor category mix">
        {bars.map((c) => (
          <li key={c.slug} className="anchors-bar-row">
            <span className="anchors-bar-cat" style={{ color: CAT_COLOR[c.slug] }}>
              <span className="legend-dot" />
              <span style={{ color: "var(--muted)" }}>{c.label}</span>
            </span>
            <span className="anchors-bar-track">
              <span
                className="anchors-bar-fill"
                style={{
                  width: `${c.count > 0 ? Math.max(4, (c.count / maxCount) * 100) : 0}%`,
                  background: CAT_COLOR[c.slug],
                }}
              />
            </span>
            <span className="anchors-bar-val">
              <b>{c.count}</b> <span className="muted">({fmtPct(c.percent)})</span>
            </span>
          </li>
        ))}
      </ul>

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

      {topAnchors.length > 0 && (
        <div className="anchors-top">
          <div className="anchors-top-head">Most-repeated anchors</div>
          <div className="anchors-donut">
            <div
              className="anchors-donut-ring"
              style={{ background: `conic-gradient(${ringStops})` }}
              role="img"
              aria-label="Share of links covered by the 5 most-repeated anchors"
            >
              <span className="anchors-donut-ctr">
                <b>{fmtPct(top5Pct)}</b>
                <span>top 5</span>
              </span>
            </div>
            <ul className="anchors-donut-legend">
              {topAnchors.map((a) => (
                <li key={`${a.slug}:${a.anchor}`}>
                  <span className="legend-dot" style={{ background: CAT_COLOR[a.slug] }} />
                  <span className="adl-phrase" title={`${a.anchor} · ${a.label}`}>
                    {a.anchor}
                  </span>
                  <span className="adl-val">
                    <b>{a.count}</b> <span className="muted">({fmtPct(a.percent)})</span>
                  </span>
                </li>
              ))}
            </ul>
          </div>
        </div>
      )}

      <p className="anchors-caveat">{data.note}</p>
    </section>
  );
}
