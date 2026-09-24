"use client";

import { useCallback, useEffect, useState } from "react";
import { useRouter } from "next/navigation";
import { api, orgApi, type BacklinkView, type DashboardResponse, type MeResponse, type ProjectView } from "../../lib/api";
import { statusColor, statusLabel } from "../../lib/status";
import "../auth.css";
import "./dashboard.css";

export default function DashboardPage() {
  const router = useRouter();
  const [me, setMe] = useState<MeResponse | null>(null);
  const [orgId, setOrgId] = useState<string | null>(null);
  const [summary, setSummary] = useState<DashboardResponse | null>(null);
  const [projects, setProjects] = useState<ProjectView[]>([]);
  const [projectId, setProjectId] = useState<string | null>(null);
  const [backlinks, setBacklinks] = useState<BacklinkView[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    api.me().then((res) => {
      if (!res.ok) {
        router.push("/login");
        return;
      }
      setMe(res.data);
      const first = res.data.organizations[0]?.id ?? null;
      setOrgId(first);
      setLoading(false);
    });
  }, [router]);

  const loadOrg = useCallback(async (org: string) => {
    const [dash, projs] = await Promise.all([orgApi.dashboard(org), orgApi.projects(org)]);
    if (dash.ok) setSummary(dash.data);
    if (projs.ok) {
      setProjects(projs.data);
      const p = projs.data[0]?.id ?? null;
      setProjectId(p);
    }
  }, []);

  useEffect(() => {
    if (orgId) void loadOrg(orgId);
  }, [orgId, loadOrg]);

  useEffect(() => {
    if (orgId && projectId) {
      orgApi.backlinks(orgId, projectId).then((res) => {
        if (res.ok) setBacklinks(res.data);
      });
    } else {
      setBacklinks([]);
    }
  }, [orgId, projectId]);

  if (loading) return <div className="auth-wrap"><p style={{ color: "var(--muted)" }}>Loading…</p></div>;
  if (!me) return null;

  const money = (rows: { currency: string; amount: string }[]) =>
    rows.length === 0 ? "—" : rows.map((r) => `${r.amount} ${r.currency}`).join(" · ");

  return (
    <main className="dash">
      <header className="dash-head">
        <div>
          <h1>Dashboard</h1>
          <p className="muted">{me.organizations[0]?.name ?? "No workspace"} · {me.user.email}</p>
        </div>
        <button className="btn" onClick={async () => { await api.logout(); router.push("/login"); }}>Log out</button>
      </header>

      {me.organizations.length === 0 ? (
        <p className="muted">No workspace yet. Create one to start monitoring.</p>
      ) : (
        <>
          <section className="tiles">
            <Tile label="MONITORED" value={summary?.monitored ?? 0} />
            <Tile label="LIVE" value={summary?.counts["live"] ?? 0} color="var(--green)" />
            <Tile label="NEEDS ATTENTION" value={summary?.needs_attention ?? 0} color="var(--yellow)" />
            <Tile label="VALUE AT RISK" value={money(summary?.value_at_risk ?? [])} />
          </section>

          <section className="panel">
            <div className="panel-head">
              <h2>Backlinks</h2>
              {projects.length > 0 && (
                <select value={projectId ?? ""} onChange={(e) => setProjectId(e.target.value)}>
                  {projects.map((p) => (
                    <option key={p.id} value={p.id}>{p.name} — {p.domain}</option>
                  ))}
                </select>
              )}
            </div>
            {backlinks.length === 0 ? (
              <p className="muted">No backlinks in this project yet.</p>
            ) : (
              <table className="bl-table">
                <thead>
                  <tr><th>Status</th><th>Source</th><th>Target</th><th>Anchor</th><th>Rel</th><th>Cost</th></tr>
                </thead>
                <tbody>
                  {backlinks.map((b) => (
                    <tr key={b.id}>
                      <td><span className="badge" style={{ color: statusColor(b.status) }}>● {statusLabel(b.status)}</span></td>
                      <td className="mono">{b.source_domain}</td>
                      <td className="mono">{b.target_domain}</td>
                      <td>{b.expected_anchor_text ?? "—"}</td>
                      <td>{b.expected_rel_type}</td>
                      <td className="mono">{b.cost ? `${b.cost} ${b.currency ?? ""}` : "—"}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            )}
          </section>
        </>
      )}
    </main>
  );
}

function Tile({ label, value, color }: { label: string; value: number | string; color?: string }) {
  return (
    <div className="tile">
      <div className="tile-label">{label}</div>
      <div className="tile-value" style={color ? { color } : undefined}>{value}</div>
    </div>
  );
}
