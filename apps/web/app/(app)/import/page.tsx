"use client";

import { useCallback, useEffect, useState } from "react";
import { useRouter } from "next/navigation";
import {
  api,
  orgApi,
  type ImportResult,
  type MeResponse,
  type ProjectView,
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
