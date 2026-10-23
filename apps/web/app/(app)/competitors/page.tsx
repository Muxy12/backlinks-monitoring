"use client";
import { PageLoading } from "@/components/PageLoading";

import { useCallback, useEffect, useState, type CSSProperties, type FormEvent } from "react";
import { useRouter } from "next/navigation";
import {
  api,
  orgApi,
  type BacklinkView,
  type CompetitorView,
  type MeResponse,
  type ProjectView,
} from "@/lib/api";
import { selectedOrgId } from "@/lib/workspace";
import { statusColor, statusLabel } from "@/lib/status";
import "@/app/auth.css";
import "../dashboard/dashboard.css";
import "../operations/operations.css";

export default function CompetitorsPage() {
  const router = useRouter();
  const [me, setMe] = useState<MeResponse | null>(null);
  const [orgId, setOrgId] = useState<string | null>(null);
  const [projects, setProjects] = useState<ProjectView[]>([]);
  const [projectId, setProjectId] = useState<string | null>(null);
  const [competitors, setCompetitors] = useState<CompetitorView[]>([]);
  const [domain, setDomain] = useState("");
  const [err, setErr] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);

  const [selected, setSelected] = useState<CompetitorView | null>(null);
  const [cbacklinks, setCbacklinks] = useState<BacklinkView[]>([]);
  const [csv, setCsv] = useState("");
  const [importMsg, setImportMsg] = useState<string | null>(null);
  const [gap, setGap] = useState<{ source_domain: string; competitor_count: number }[]>([]);
  const [gapMin, setGapMin] = useState(1);

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

  const loadCompetitors = useCallback(async (org: string, project: string) => {
    const res = await orgApi.competitors(org, project);
    if (res.ok) setCompetitors(res.data);
  }, []);

  useEffect(() => {
    if (!orgId) return;
    orgApi.projects(orgId).then((res) => {
      if (res.ok) {
        setProjects(res.data);
        setProjectId((p) => p ?? res.data[0]?.id ?? null);
      }
    });
  }, [orgId]);

  useEffect(() => {
    if (orgId && projectId) void loadCompetitors(orgId, projectId);
  }, [orgId, projectId, loadCompetitors]);

  useEffect(() => {
    if (orgId && projectId) {
      orgApi.backlinkGap(orgId, projectId, gapMin).then((r) => {
        if (r.ok) setGap(r.data);
      });
    }
  }, [orgId, projectId, gapMin, competitors]);

  async function addCompetitor(e: FormEvent) {
    e.preventDefault();
    if (!orgId || !projectId) return;
    const d = domain.trim();
    if (!d) return;
    setErr(null);
    const res = await orgApi.createCompetitor(orgId, projectId, d);
    if (!res.ok) {
      setErr(res.detail || "Could not add competitor.");
      return;
    }
    setDomain("");
    await loadCompetitors(orgId, projectId);
  }

  async function removeCompetitor(id: string) {
    if (!orgId) return;
    const res = await orgApi.deleteCompetitor(orgId, id);
    if (res.ok && projectId) {
      if (selected?.id === id) setSelected(null);
      await loadCompetitors(orgId, projectId);
    }
  }

  async function openCompetitor(c: CompetitorView) {
    setSelected(c);
    setCbacklinks([]);
    setImportMsg(null);
    setCsv("");
    if (orgId) {
      const res = await orgApi.competitorBacklinks(orgId, c.id);
      if (res.ok) setCbacklinks(res.data);
    }
  }

  async function discoverCompetitor() {
    if (!orgId || !projectId || !selected) return;
    setImportMsg("Running DataForSEO…");
    const res = await orgApi.runDiscovery(orgId, projectId, "dataforseo", selected.id);
    if (!res.ok) {
      setImportMsg(res.detail || "DataForSEO failed (is it configured?)");
      return;
    }
    setImportMsg(`DataForSEO: ${res.data.discovered} found, ${res.data.new} new.`);
    await openCompetitor(selected);
  }

  async function runImport() {
    if (!orgId || !selected) return;
    const rows = csv
      .split(/\r?\n/)
      .map((l) => l.trim())
      .filter(Boolean)
      .map((l) => {
        const [source_url, target_url, anchor, rel] = l.split(",").map((s) => s?.trim());
        return { source_url, target_url, anchor, rel };
      })
      .filter((r) => r.source_url && r.target_url);
    if (rows.length === 0) {
      setImportMsg("Paste rows: source_url,target_url,anchor,rel");
      return;
    }
    const res = await orgApi.importCompetitorBacklinks(orgId, selected.id, rows);
    if (!res.ok) {
      setImportMsg(res.detail || "Import failed.");
      return;
    }
    setImportMsg(`Imported ${res.data.imported} (${res.data.new} new, ${res.data.skipped} skipped).`);
    setCsv("");
    await openCompetitor(selected);
  }

  if (loading) return <PageLoading />;
  if (!me) return null;

  return (
    <main className="dash">
      <header className="page-head">
        <h1>Competitors</h1>
        <p className="muted">Track competitor domains and the backlinks they earn.</p>
      </header>

      {me.organizations.length === 0 ? (
        <p className="muted">Create a workspace first.</p>
      ) : (
        <>
        <div className="ops-grid">
          <section className="panel">
            <div className="panel-head">
              <h2>Competitors</h2>
              {projects.length > 0 && (
                <select value={projectId ?? ""} onChange={(e) => setProjectId(e.target.value)}>
                  {projects.map((p) => (
                    <option key={p.id} value={p.id}>{p.name}, {p.domain}</option>
                  ))}
                </select>
              )}
            </div>
            {projects.length === 0 ? (
              <p className="muted">Create a project first.</p>
            ) : competitors.length === 0 ? (
              <p className="muted">No competitors yet for this project.</p>
            ) : (
              <table className="bl-table">
                <thead><tr><th>Domain</th><th></th></tr></thead>
                <tbody>
                  {competitors.map((c) => (
                    <tr key={c.id} className="row-click" onClick={() => void openCompetitor(c)}>
                      <td className="mono">{c.normalized_domain}</td>
                      <td style={{ textAlign: "right" }}>
                        <button
                          className="btn sm"
                          onClick={(e) => { e.stopPropagation(); void removeCompetitor(c.id); }}
                        >
                          Remove
                        </button>
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            )}
            <form onSubmit={addCompetitor} className="ops-form" style={{ marginTop: 16 }}>
              <label className="auth-field">
                <span>Add competitor domain</span>
                <input value={domain} onChange={(e) => setDomain(e.target.value)} placeholder="competitor.com" disabled={!projectId} />
              </label>
              {err && <div className="auth-error" role="alert">{err}</div>}
              <button className="btn primary" type="submit" disabled={!projectId}>Add competitor</button>
            </form>
          </section>

          <section className="panel">
            {!selected ? (
              <p className="muted">Select a competitor to see its backlinks and import discovered links.</p>
            ) : (
              <>
                <div className="panel-head">
                  <h2>{selected.normalized_domain}</h2>
                  <span className="muted">{cbacklinks.length} backlinks</span>
                </div>
                <div style={{ marginBottom: 16 }}>
                  <p className="muted" style={{ marginBottom: 6 }}>
                    Import discovered backlinks, one per line: <code>source_url,target_url,anchor,rel</code>
                  </p>
                  <textarea
                    className="bl-search"
                    style={{ width: "100%", minHeight: 90, fontFamily: "ui-monospace, monospace" }}
                    value={csv}
                    onChange={(e) => setCsv(e.target.value)}
                    placeholder={`https://news.com/post,https://${selected.normalized_domain}/page,great tool,dofollow`}
                  />
                  <div style={{ display: "flex", gap: 10, alignItems: "center", marginTop: 8, flexWrap: "wrap" }}>
                    <button className="btn primary" onClick={() => void runImport()}>Import</button>
                    <button className="btn sm" onClick={() => void discoverCompetitor()}>Discover via DataForSEO</button>
                    {importMsg && <span className="muted">{importMsg}</span>}
                  </div>
                </div>
                {cbacklinks.length === 0 ? (
                  <p className="muted">No backlinks recorded for this competitor yet.</p>
                ) : (
                  <table className="bl-table">
                    <thead><tr><th>Status</th><th>Source</th><th>Anchor</th><th>Rel</th></tr></thead>
                    <tbody>
                      {cbacklinks.map((b) => (
                        <tr key={b.id}>
                          <td><span className="badge" style={{ "--st": statusColor(b.status) } as CSSProperties}><span className="dot" />{statusLabel(b.status)}</span></td>
                          <td className="mono" title={b.source_url}>{b.source_domain}</td>
                          <td>{b.detected_anchor_text ?? b.expected_anchor_text ?? "-"}</td>
                          <td>{b.expected_rel_type}</td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                )}
              </>
            )}
          </section>
        </div>

        <section className="panel" style={{ marginTop: 16 }}>
          <div className="panel-head">
            <h2>Backlink gap</h2>
            <select value={gapMin} onChange={(e) => setGapMin(Number(e.target.value))}>
              <option value={1}>Links ≥ 1 competitor</option>
              <option value={2}>Links ≥ 2 competitors</option>
              <option value={3}>Links ≥ 3 competitors</option>
            </select>
          </div>
          <p className="muted">Domains that link to your competitors but not to you, outreach opportunities.</p>
          {gap.length === 0 ? (
            <p className="muted">No gap domains yet (import competitor backlinks to populate this).</p>
          ) : (
            <table className="bl-table">
              <thead><tr><th>Referring domain</th><th>Competitors linked</th></tr></thead>
              <tbody>
                {gap.map((g) => (
                  <tr key={g.source_domain}>
                    <td className="mono">{g.source_domain}</td>
                    <td><b>{g.competitor_count}</b></td>
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
