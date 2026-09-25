"use client";

import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useRouter } from "next/navigation";
import dynamic from "next/dynamic";
import { api, orgApi, type GraphResponse, type MeResponse, type ProjectView } from "@/lib/api";
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

  useEffect(() => {
    api.me().then((res) => {
      if (!res.ok) {
        router.push("/login");
        return;
      }
      setMe(res.data);
      setOrgId(res.data.organizations[0]?.id ?? null);
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
    const nodes = data.nodes
      .filter((n) => keep(n.tier))
      .map((n) => ({
        id: n.id,
        label: n.label,
        domain: n.domain,
        tier: n.tier,
        type: n.type,
        __match: query.length > 0 && n.label.toLowerCase().includes(query),
      }));
    const nodeIds = new Set(nodes.map((n) => n.id));
    const links = data.edges
      .filter((e) => nodeIds.has(e.source) && nodeIds.has(e.target))
      .map((e) => ({ source: e.source, target: e.target, status: e.status, tier: e.tier, backlink_id: e.backlink_id }));
    return { nodes, links };
  }, [data, tierFilter, q]);

  if (loading) return <div className="auth-wrap"><p style={{ color: "var(--muted)" }}>Loading…</p></div>;
  if (!me) return null;

  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const commonProps: any = {
    graphData,
    width: size.w,
    height: size.h,
    backgroundColor: "rgba(0,0,0,0)",
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    nodeLabel: (n: any) => `${n.label} (tier ${n.tier})`,
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    nodeColor: (n: any) => (n.__match ? "#fbbf24" : tierColor(n.tier)),
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    nodeVal: (n: any) => (n.tier === 0 ? 14 : n.tier === 1 ? 5 : 2),
    linkColor: () => "rgba(255,255,255,0.18)",
    linkDirectionalArrowLength: 3,
    linkDirectionalArrowRelPos: 1,
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    onNodeClick: (n: any) => {
      setSelEdge(null);
      setSelNode({ id: n.id, label: n.label, domain: n.domain, tier: n.tier, type: n.type });
    },
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    onLinkClick: (l: any) => {
      setSelNode(null);
      const src = typeof l.source === "object" ? l.source.id : l.source;
      const tgt = typeof l.target === "object" ? l.target.id : l.target;
      setSelEdge({ source: src, target: tgt, status: l.status, tier: l.tier, backlink_id: l.backlink_id });
    },
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
            <span><i style={{ background: tierColor(0) }} /> Target</span>
            <span><i style={{ background: tierColor(1) }} /> Tier 1</span>
            <span><i style={{ background: tierColor(2) }} /> Tier 2</span>
          </div>

          <div className="graph-stage" ref={wrapRef}>
            {graphData.nodes.length === 0 ? (
              <p className="muted" style={{ padding: 24 }}>No backlinks to graph yet for this project.</p>
            ) : mode === "2d" ? (
              <ForceGraph2D {...commonProps} />
            ) : (
              <ForceGraph3D {...commonProps} />
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
