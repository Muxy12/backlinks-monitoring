"use client";

import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useRouter } from "next/navigation";
import dynamic from "next/dynamic";
import { api, orgApi, type GraphResponse, type MeResponse, type ProjectView } from "@/lib/api";
import { selectedOrgId } from "@/lib/workspace";
import "@/app/auth.css";
import "../dashboard/dashboard.css";
import "./graph.css";

// Heavy WebGL/canvas libs: client-only + lazy so they never touch SSR or the
// main bundle. 2D is the default (mobile / weak GPUs); 3D loads on demand.
// eslint-disable-next-line @typescript-eslint/no-explicit-any
const ForceGraph2D = dynamic(() => import("react-force-graph-2d"), { ssr: false }) as any;
// eslint-disable-next-line @typescript-eslint/no-explicit-any
const ForceGraph3D = dynamic(() => import("react-force-graph-3d"), { ssr: false }) as any;

const TIER_COLOR: Record<number, string> = {
  0: "#a78bfa", // target (root)
  1: "#36d399", // tier 1
  2: "#60a5fa", // tier 2
};
function tierColor(tier: number): string {
  return TIER_COLOR[tier] ?? "#8b949e";
}

// Edges are colored by the backlink's health, so the graph reads at a glance.
const STATUS_COLOR: Record<string, string> = {
  live: "#36d399",
  lost: "#fb7185",
  suspected_lost: "#fb7185",
  nofollow: "#fbbf24",
  changed: "#fb923c",
  redirected: "#fb923c",
  source_error: "#f472b6",
  target_error: "#f472b6",
  blocked: "#f472b6",
  pending: "#8b949e",
  unknown: "#8b949e",
};
function statusColor(status: string): string {
  return STATUS_COLOR[status] ?? "#8b949e";
}
const LIVE = new Set(["live", ""]);

// Radius grows with tier importance and in-degree (how many links point at it),
// so hub referring domains stand out.
function nodeRadius(tier: number, deg: number): number {
  const base = tier === 0 ? 9 : tier === 1 ? 4.5 : 3;
  return base + Math.min(6, Math.sqrt(deg) * 1.1);
}

function isLightTheme(): boolean {
  return typeof document !== "undefined" && document.documentElement.getAttribute("data-theme") === "light";
}

// "#rrggbb" + alpha → "rgba(r,g,b,a)".
function withAlpha(hex: string, a: number): string {
  const m = /^#?([0-9a-f]{6})$/i.exec(hex);
  if (!m) return hex;
  const n = parseInt(m[1], 16);
  return `rgba(${(n >> 16) & 255}, ${(n >> 8) & 255}, ${n & 255}, ${a})`;
}

type SelNode = { id: string; label: string; domain: string; tier: number; type: string } | null;
type SelEdge = { source: string; target: string; status: string; tier: number; backlink_id: string } | null;

export default function GraphPage() {
  const router = useRouter();
  const [me, setMe] = useState<MeResponse | null>(null);
  const [orgId, setOrgId] = useState<string | null>(null);
  const [projects, setProjects] = useState<ProjectView[]>([]);
  const [projectId, setProjectId] = useState<string | null>(null);
  const [view, setView] = useState<"domain" | "url">("domain");
  const [mode, setMode] = useState<"2d" | "3d">("2d");
  const [tierFilter, setTierFilter] = useState<number>(-1); // -1 = all
  const [q, setQ] = useState("");
  const [data, setData] = useState<GraphResponse | null>(null);
  const [loading, setLoading] = useState(true);
  const [selNode, setSelNode] = useState<SelNode>(null);
  const [selEdge, setSelEdge] = useState<SelEdge>(null);

  const wrapRef = useRef<HTMLDivElement | null>(null);
  const [size, setSize] = useState({ w: 800, h: 520 });
  // Hovered node id — kept in a ref so the per-frame canvas renderer can read it
  // without forcing a React re-render.
  const hoverRef = useRef<string | null>(null);

  useEffect(() => {
    api.me().then((res) => {
      if (!res.ok) {
        router.push("/login");
        return;
      }
      setMe(res.data);
      setOrgId(selectedOrgId(res.data.organizations));
      setLoading(false);
    });
  }, [router]);

  useEffect(() => {
    if (!orgId) return;
    orgApi.projects(orgId).then((res) => {
      if (res.ok) {
        setProjects(res.data);
        setProjectId((p) => p ?? res.data[0]?.id ?? null);
      }
    });
  }, [orgId]);

  const loadGraph = useCallback(async (org: string, project: string, v: "domain" | "url") => {
    const res = await orgApi.backlinkGraph(org, project, v, 3000);
    if (res.ok) setData(res.data);
  }, []);

  useEffect(() => {
    if (orgId && projectId) void loadGraph(orgId, projectId, view);
  }, [orgId, projectId, view, loadGraph]);

  useEffect(() => {
    if (!wrapRef.current) return;
    const el = wrapRef.current;
    const ro = new ResizeObserver(() => {
      setSize({ w: el.clientWidth, h: Math.max(420, Math.round(window.innerHeight * 0.62)) });
    });
    ro.observe(el);
    return () => ro.disconnect();
  }, [data, mode]);

  // Build the force-graph dataset from the API response, applying filters.
  const graphData = useMemo(() => {
    if (!data) return { nodes: [], links: [] };
    const keep = (tier: number) => tierFilter === -1 || tier === tierFilter || tier === 0;
    const query = q.trim().toLowerCase();
    const kept = data.nodes.filter((n) => keep(n.tier));
    const nodeIds = new Set(kept.map((n) => n.id));
    const links = data.edges
      .filter((e) => nodeIds.has(e.source) && nodeIds.has(e.target))
      .map((e) => ({ source: e.source, target: e.target, status: e.status, tier: e.tier, backlink_id: e.backlink_id }));
    // In-degree per node → drives node size (hubs look bigger).
    const deg: Record<string, number> = {};
    for (const l of links) deg[l.target] = (deg[l.target] ?? 0) + 1;
    const nodes = kept.map((n) => ({
      id: n.id,
      label: n.label,
      domain: n.domain,
      tier: n.tier,
      type: n.type,
      deg: deg[n.id] ?? 0,
      __match: query.length > 0 && n.label.toLowerCase().includes(query),
    }));
    return { nodes, links };
  }, [data, tierFilter, q]);

  // Adjacency for hover highlighting (dim everything not connected to the
  // hovered node).
  const neighbors = useMemo(() => {
    const idOf = (v: unknown) => (typeof v === "object" && v ? (v as { id: string }).id : (v as string));
    const m: Record<string, Set<string>> = {};
    for (const l of graphData.links) {
      const s = idOf(l.source);
      const t = idOf(l.target);
      (m[s] ??= new Set()).add(t);
      (m[t] ??= new Set()).add(s);
    }
    return m;
  }, [graphData]);

  if (loading) return <div className="auth-wrap"><p style={{ color: "var(--muted)" }}>Loading…</p></div>;
  if (!me) return null;

  const linkId = (l: { source: unknown; target: unknown }) => {
    const s = typeof l.source === "object" ? (l.source as { id: string }).id : (l.source as string);
    const t = typeof l.target === "object" ? (l.target as { id: string }).id : (l.target as string);
    return { s, t };
  };
  const connectedToHover = (s: string, t: string) => {
    const h = hoverRef.current;
    return !h || s === h || t === h;
  };

  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const baseProps: any = {
    graphData,
    width: size.w,
    height: size.h,
    backgroundColor: "rgba(0,0,0,0)",
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    nodeLabel: (n: any) => `${n.label} · tier ${n.tier}${n.deg ? ` · ${n.deg} in` : ""}`,
    linkCurvature: 0.12,
    linkDirectionalArrowLength: 2.5,
    linkDirectionalArrowRelPos: 1,
    // Animate particles along live links only — a subtle sense of "flow".
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    linkDirectionalParticles: (l: any) => (LIVE.has(l.status) ? 2 : 0),
    linkDirectionalParticleWidth: 1.8,
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    linkDirectionalParticleColor: (l: any) => statusColor(l.status),
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    linkColor: (l: any) => {
      const { s, t } = linkId(l);
      const on = connectedToHover(s, t);
      const c = statusColor(l.status);
      return withAlpha(c, on ? 0.6 : 0.08);
    },
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    linkWidth: (l: any) => {
      const { s, t } = linkId(l);
      const hot = hoverRef.current && connectedToHover(s, t);
      return (l.tier === 1 ? 1.1 : 0.6) * (hot ? 2.4 : 1);
    },
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    onNodeHover: (n: any) => {
      hoverRef.current = n ? n.id : null;
      if (wrapRef.current) wrapRef.current.style.cursor = n ? "pointer" : "grab";
    },
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    onNodeClick: (n: any) => {
      setSelEdge(null);
      setSelNode({ id: n.id, label: n.label, domain: n.domain, tier: n.tier, type: n.type });
    },
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    onLinkClick: (l: any) => {
      setSelNode(null);
      const { s, t } = linkId(l);
      setSelEdge({ source: s, target: t, status: l.status, tier: l.tier, backlink_id: l.backlink_id });
    },
  };

  // 2D: fully custom node — soft glow, degree-scaled radius, ring for
  // root/match/hover, and a label for the root, hovered, or when zoomed in.
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const twoDProps: any = {
    ...baseProps,
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    nodeCanvasObject: (n: any, ctx: CanvasRenderingContext2D, scale: number) => {
      const h = hoverRef.current;
      const focused = !h || n.id === h || neighbors[h]?.has(n.id);
      const r = nodeRadius(n.tier, n.deg);
      const color = n.__match ? "#fbbf24" : tierColor(n.tier);
      ctx.globalAlpha = focused ? 1 : 0.25;
      // glow
      ctx.save();
      ctx.shadowColor = color;
      ctx.shadowBlur = (n.tier === 0 ? 16 : 7) / Math.max(1, scale * 0.6);
      ctx.beginPath();
      ctx.arc(n.x, n.y, r, 0, 2 * Math.PI);
      ctx.fillStyle = color;
      ctx.fill();
      ctx.restore();
      // ring
      if (n.__match || n.id === h || n.tier === 0) {
        ctx.beginPath();
        ctx.arc(n.x, n.y, r + 2.5, 0, 2 * Math.PI);
        ctx.strokeStyle = n.__match ? "#fbbf24" : isLightTheme() ? "rgba(15,23,42,.55)" : "rgba(255,255,255,.75)";
        ctx.lineWidth = 1.4 / scale;
        ctx.stroke();
      }
      // label
      const showLabel = n.tier === 0 || n.id === h || scale > 2.4;
      if (showLabel && focused) {
        const label = n.label.length > 30 ? n.label.slice(0, 29) + "…" : n.label;
        const fs = Math.max(11 / scale, 3.5);
        ctx.font = `600 ${fs}px ui-sans-serif, system-ui, -apple-system, sans-serif`;
        ctx.textAlign = "center";
        ctx.textBaseline = "top";
        ctx.fillStyle = isLightTheme() ? "#0f172a" : "#e6e9ef";
        ctx.fillText(label, n.x, n.y + r + 2 / scale);
      }
      ctx.globalAlpha = 1;
    },
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    nodePointerAreaPaint: (n: any, color: string, ctx: CanvasRenderingContext2D) => {
      ctx.fillStyle = color;
      ctx.beginPath();
      ctx.arc(n.x, n.y, nodeRadius(n.tier, n.deg) + 2, 0, 2 * Math.PI);
      ctx.fill();
    },
  };

  // 3D: keep the built-in spheres but colored/sized like the 2D nodes.
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const threeDProps: any = {
    ...baseProps,
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    nodeColor: (n: any) => (n.__match ? "#fbbf24" : tierColor(n.tier)),
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    nodeVal: (n: any) => (n.tier === 0 ? 16 : n.tier === 1 ? 5 : 2) + n.deg,
    nodeOpacity: 0.92,
    nodeResolution: 14,
  };

  return (
    <main className="dash">
      <header className="page-head">
        <h1>Backlink graph</h1>
        <p className="muted">Your monitored domain at the centre, Tier 1 around it, Tier 2 beyond.</p>
      </header>

      {me.organizations.length === 0 ? (
        <p className="muted">Create a workspace first.</p>
      ) : projects.length === 0 ? (
        <p className="muted">Create a project first.</p>
      ) : (
        <>
          <div className="graph-toolbar">
            <select value={projectId ?? ""} onChange={(e) => setProjectId(e.target.value)}>
              {projects.map((p) => (
                <option key={p.id} value={p.id}>{p.name} — {p.domain}</option>
              ))}
            </select>
            <select value={view} onChange={(e) => setView(e.target.value as "domain" | "url")}>
              <option value="domain">Domain view</option>
              <option value="url">URL view</option>
            </select>
            <select value={tierFilter} onChange={(e) => setTierFilter(Number(e.target.value))}>
              <option value={-1}>All tiers</option>
              <option value={1}>Tier 1</option>
              <option value={2}>Tier 2</option>
            </select>
            <input className="bl-search" placeholder="Highlight domain/url" value={q} onChange={(e) => setQ(e.target.value)} />
            <div className="mode-toggle">
              <button className={`btn sm${mode === "2d" ? " active" : ""}`} onClick={() => setMode("2d")}>2D</button>
              <button className={`btn sm${mode === "3d" ? " active" : ""}`} onClick={() => setMode("3d")}>3D</button>
            </div>
            <span className="muted">{graphData.nodes.length} nodes · {graphData.links.length} edges{data?.truncated ? " (truncated)" : ""}</span>
          </div>

          <div className="graph-legend">
            <span className="legend-group">
              <i style={{ background: tierColor(0) }} /> Target
              <i style={{ background: tierColor(1) }} /> Tier 1
              <i style={{ background: tierColor(2) }} /> Tier 2
            </span>
            <span className="legend-sep" />
            <span className="legend-group">
              <i style={{ background: statusColor("live") }} /> Live
              <i style={{ background: statusColor("lost") }} /> Lost
              <i style={{ background: statusColor("nofollow") }} /> Nofollow
              <i style={{ background: statusColor("redirected") }} /> Changed
            </span>
          </div>

          <div className="graph-stage" ref={wrapRef}>
            {graphData.nodes.length === 0 ? (
              <p className="muted" style={{ padding: 24 }}>No backlinks to graph yet for this project.</p>
            ) : mode === "2d" ? (
              <ForceGraph2D {...twoDProps} />
            ) : (
              <ForceGraph3D {...threeDProps} />
            )}
          </div>

          {(selNode || selEdge) && (
            <section className="panel graph-detail">
              {selNode && (
                <>
                  <div className="panel-head"><h2>Node</h2><button className="btn sm" onClick={() => setSelNode(null)}>Close</button></div>
                  <dl className="detail-grid">
                    <div><dt>Label</dt><dd className="mono br">{selNode.label}</dd></div>
                    <div><dt>Domain</dt><dd className="mono">{selNode.domain}</dd></div>
                    <div><dt>Tier</dt><dd>{selNode.tier}</dd></div>
                    <div><dt>Type</dt><dd>{selNode.type}</dd></div>
                    <div><dt>Incoming</dt><dd>{graphData.links.filter((l) => (typeof l.target === "string" ? l.target : "") === selNode.id).length}</dd></div>
                    <div><dt>Outgoing</dt><dd>{graphData.links.filter((l) => (typeof l.source === "string" ? l.source : "") === selNode.id).length}</dd></div>
                  </dl>
                </>
              )}
              {selEdge && (
                <>
                  <div className="panel-head"><h2>Edge</h2><button className="btn sm" onClick={() => setSelEdge(null)}>Close</button></div>
                  <dl className="detail-grid">
                    <div><dt>Source</dt><dd className="mono br">{selEdge.source}</dd></div>
                    <div><dt>Target</dt><dd className="mono br">{selEdge.target}</dd></div>
                    <div><dt>Tier</dt><dd>{selEdge.tier}</dd></div>
                    <div><dt>Status</dt><dd>{selEdge.status}</dd></div>
                  </dl>
                </>
              )}
            </section>
          )}
        </>
      )}
    </main>
  );
}
