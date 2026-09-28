"use client";

import { useCallback, useEffect, useState } from "react";
import { useRouter } from "next/navigation";
import {
  api,
  orgApi,
  type ImportResult,
  type MeResponse,
  type ProjectView,
  type SeedView,
} from "@/lib/api";
import "@/app/auth.css";
import "../dashboard/dashboard.css";
import "../operations/operations.css";

const SAMPLE = `source_url,target_url,anchor,rel,cost,currency
https://a-publisher.com/post,https://your-site.com/,best tools,dofollow,120,USD`;

export default function ImportPage() {
  const router = useRouter();
  const [me, setMe] = useState<MeResponse | null>(null);
  const [orgId, setOrgId] = useState<string | null>(null);
  const [projects, setProjects] = useState<ProjectView[]>([]);
  const [projectId, setProjectId] = useState("");
  const [csv, setCsv] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [result, setResult] = useState<ImportResult | null>(null);
  const [loading, setLoading] = useState(true);
  const [seedText, setSeedText] = useState("");
  const [seeds, setSeeds] = useState<SeedView[]>([]);
  const [seedMsg, setSeedMsg] = useState<string | null>(null);
  const [discBusy, setDiscBusy] = useState(false);

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

  const loadProjects = useCallback(async (org: string) => {
    const res = await orgApi.projects(org);
    if (res.ok) {
      setProjects(res.data);
      setProjectId(res.data[0]?.id ?? "");
    }
  }, []);

  useEffect(() => {
    if (orgId) void loadProjects(orgId);
  }, [orgId, loadProjects]);

  useEffect(() => {
    if (orgId && projectId) {
      orgApi.discoverySeeds(orgId, projectId).then((r) => {
        if (r.ok) setSeeds(r.data);
      });
    }
  }, [orgId, projectId]);

  async function addSeeds() {
    if (!orgId || !projectId) return;
    const domains = seedText
      .split(/\r?\n/)
      .map((l) => (l.split(/[,\t]/)[0] ?? "").trim().replace(/^"|"$/g, ""))
      .filter(Boolean)
      .filter((d) => !/^(site|top linking sites|domain|linking sites)$/i.test(d));
    if (domains.length === 0) {
      setSeedMsg("Paste domains, or a GSC 'Top linking sites' export (first column).");
      return;
    }
    setDiscBusy(true);
    const res = await orgApi.importSeeds(orgId, projectId, domains, "gsc");
    setDiscBusy(false);
    if (!res.ok) {
      setSeedMsg(res.detail || "Import failed.");
      return;
    }
    setSeedMsg(`Added ${res.data.added} (${res.data.skipped} skipped). The crawler will expand them into real backlinks.`);
    setSeedText("");
    const s = await orgApi.discoverySeeds(orgId, projectId);
    if (s.ok) setSeeds(s.data);
  }

  async function runProvider(provider: string) {
    if (!orgId || !projectId) return;
    setDiscBusy(true);
    setSeedMsg(`Running ${provider}…`);
    const res = await orgApi.runDiscovery(orgId, projectId, provider);
    setDiscBusy(false);
    if (!res.ok) {
      setSeedMsg(res.detail || `${provider} failed`);
      return;
    }
    setSeedMsg(`${provider}: ${res.data.discovered} found, ${res.data.new} new backlinks.`);
    const s = await orgApi.discoverySeeds(orgId, projectId);
    if (s.ok) setSeeds(s.data);
  }

  async function onFile(e: React.ChangeEvent<HTMLInputElement>) {
    const f = e.target.files?.[0];
    if (f) setCsv(await f.text());
  }

  async function onSubmit(e: React.FormEvent) {
    e.preventDefault();
    setError(null);
    setResult(null);
    if (!orgId || !projectId) {
      setError("Pick a project.");
      return;
    }
    if (!csv.trim()) {
      setError("Paste or upload some CSV first.");
      return;
    }
    setBusy(true);
    const res = await orgApi.importCsv(orgId, projectId, csv);
    setBusy(false);
    if (!res.ok) {
      setError(res.detail);
      return;
    }
    setResult(res.data);
  }

  if (loading) return <div className="auth-wrap"><p style={{ color: "var(--muted)" }}>Loading…</p></div>;
  if (!me) return null;

  const targetDomain = projects.find((p) => p.id === projectId)?.domain ?? "";

  return (
    <main className="dash">
      <header className="page-head">
        <h1>Import backlinks</h1>
        <p className="muted">CSV — generic, Ahrefs, or Semrush headers. Duplicates are skipped.</p>
      </header>

      {!orgId ? (
        <p className="muted">Create a workspace first.</p>
      ) : projects.length === 0 ? (
        <p className="muted">Create a project before importing.</p>
      ) : (
        <>
        <div className="ops-grid">
          <section className="panel">
            <div className="panel-head"><h2>Upload</h2></div>
            <form onSubmit={onSubmit} className="ops-form">
              {error && <div className="auth-error">{error}</div>}
              <label className="auth-field">
                <span>Project</span>
                <select value={projectId} onChange={(e) => setProjectId(e.target.value)} required>
                  {projects.map((p) => (
                    <option key={p.id} value={p.id}>{p.name} — {p.domain}</option>
                  ))}
                </select>
              </label>
              <label className="auth-field">
                <span>CSV file</span>
                <input type="file" accept=".csv,text/csv" onChange={onFile} />
              </label>
              <label className="auth-field">
                <span>…or paste CSV</span>
                <textarea
                  className="csv-area"
                  value={csv}
                  onChange={(e) => setCsv(e.target.value)}
                  rows={10}
                  placeholder={SAMPLE}
                />
              </label>
              <button className="auth-btn" type="submit" disabled={busy}>
                {busy ? "Importing…" : "Import"}
              </button>
            </form>
          </section>

          <section className="panel">
            <div className="panel-head"><h2>Result</h2></div>
            {!result ? (
              <p className="muted">
                Import a file to see how many rows were added, skipped as duplicates, or rejected.
              </p>
            ) : (
              <>
                <div className="rollup">
                  <Stat label="Rows" v={result.total} />
                  <Stat label="Imported" v={result.imported} />
                  <Stat label="Duplicates" v={result.duplicates} />
                  <Stat label="Errors" v={result.errors.length} />
                </div>
                {result.errors.length > 0 && (
                  <table className="bl-table" style={{ marginTop: 12 }}>
                    <thead>
                      <tr><th>Row</th><th>Problem</th></tr>
                    </thead>
                    <tbody>
                      {result.errors.map((er, i) => (
                        <tr key={i}>
                          <td className="mono">{er.row_number}</td>
                          <td>{er.message}</td>
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
            <h2>Discover from referring domains</h2>
          </div>

          {/* Link the discovery to a project first — everything found is attached
              to this project's site. */}
          <label className="auth-field">
            <span>Target project — discovered backlinks are linked to this site</span>
            <select value={projectId} onChange={(e) => setProjectId(e.target.value)}>
              {projects.map((p) => (
                <option key={p.id} value={p.id}>{p.name} — {p.domain}</option>
              ))}
            </select>
          </label>
          {targetDomain && (
            <p className="muted" style={{ margin: "2px 0 4px" }}>
              Finding backlinks that point to <b className="mono">{targetDomain}</b>.
            </p>
          )}

          <h3 className="disc-h">Automatic discovery</h3>
          <p className="muted" style={{ marginBottom: 8 }}>
            Query a backlink index for pages pointing to <b className="mono">{targetDomain || "the project"}</b>.
            No paste needed. Bing is free (your own verified site); DataForSEO is paid and covers any domain.
          </p>
          <div style={{ display: "flex", gap: 10, alignItems: "center", flexWrap: "wrap" }}>
            <button className="btn" disabled={!projectId || discBusy} onClick={() => void runProvider("dataforseo")}>
              Discover via DataForSEO
            </button>
            <button className="btn" disabled={!projectId || discBusy} onClick={() => void runProvider("bing")}>
              Discover via Bing (free)
            </button>
          </div>

          <h3 className="disc-h" style={{ marginTop: 18 }}>Seed &amp; crawl known domains</h3>
          <p className="muted" style={{ marginBottom: 8 }}>
            Paste referring <b>domains</b> (one per line) — or a Google Search Console
            “Top linking sites” export (first column). Backlio crawls each (SSRF-safe, bounded)
            to find the actual pages that link to <b className="mono">{targetDomain || "your site"}</b>.
          </p>
          <textarea
            className="csv-area"
            rows={5}
            value={seedText}
            onChange={(e) => setSeedText(e.target.value)}
            placeholder={"forbes.com\ntechcrunch.com\nsomeblog.com"}
          />
          <div style={{ marginTop: 8 }}>
            <button className="btn primary" disabled={!projectId || discBusy} onClick={() => void addSeeds()}>
              Add &amp; crawl
            </button>
          </div>

          {seedMsg && <p className="muted" style={{ marginTop: 10 }}>{seedMsg}</p>}

          {seeds.length > 0 && (
            <table className="bl-table" style={{ marginTop: 14 }}>
              <thead><tr><th>Domain</th><th>Status</th><th>Pages</th><th>Backlinks</th></tr></thead>
              <tbody>
                {seeds.map((s) => (
                  <tr key={s.id}>
                    <td className="mono">{s.normalized_domain}</td>
                    <td>{s.status}</td>
                    <td className="mono">{s.pages_crawled}</td>
                    <td className="mono">{s.backlinks_found}</td>
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

function Stat({ label, v }: { label: string; v: number | string }) {
  return (
    <div className="stat">
      <div className="stat-label">{label}</div>
      <div className="stat-value">{v}</div>
    </div>
  );
}
