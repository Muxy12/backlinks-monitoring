"use client";

import { useCallback, useEffect, useState } from "react";
import { useRouter } from "next/navigation";
import { api, orgApi, type ClientView, type MeResponse, type ProjectView } from "../../lib/api";
import "../auth.css";
import "../dashboard/dashboard.css";
import "../operations/operations.css";

export default function ProjectsPage() {
  const router = useRouter();
  const [me, setMe] = useState<MeResponse | null>(null);
  const [orgId, setOrgId] = useState<string | null>(null);
  const [projects, setProjects] = useState<ProjectView[]>([]);
  const [clients, setClients] = useState<ClientView[]>([]);
  const [name, setName] = useState("");
  const [domain, setDomain] = useState("");
  const [clientId, setClientId] = useState("");
  const [hours, setHours] = useState("168"); // 7 days
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
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

  const load = useCallback(async (org: string) => {
    const [p, c] = await Promise.all([orgApi.projects(org), orgApi.clients(org)]);
    if (p.ok) setProjects(p.data);
    if (c.ok) setClients(c.data);
  }, []);

  useEffect(() => {
    if (orgId) void load(orgId);
  }, [orgId, load]);

  const clientName = (id: string | null) =>
    id ? (clients.find((c) => c.id === id)?.name ?? "—") : "—";

  async function create(e: React.FormEvent) {
    e.preventDefault();
    setError(null);
    if (!orgId) return;
    setBusy(true);
    const res = await orgApi.createProject(orgId, {
      name: name.trim(),
      domain: domain.trim(),
      check_frequency_hours: Number(hours) || undefined,
      client_id: clientId || undefined,
    });
    setBusy(false);
    if (!res.ok) {
      setError(res.detail);
      return;
    }
    setName("");
    setDomain("");
    setClientId("");
    void load(orgId);
  }

  async function reassign(projectId: string, newClient: string) {
    if (!orgId) return;
    const res = await orgApi.updateProject(orgId, projectId, {
      client_id: newClient === "" ? null : newClient,
    });
    if (res.ok) void load(orgId);
  }

  if (loading) return <div className="auth-wrap"><p style={{ color: "var(--muted)" }}>Loading…</p></div>;
  if (!me) return null;

  return (
    <main className="dash">
      <header className="dash-head">
        <div>
          <h1>Projects</h1>
          <p className="muted">The sites you monitor backlinks for.</p>
        </div>
        <a className="btn" href="/dashboard">← Dashboard</a>
      </header>

      {!orgId ? (
        <p className="muted">Create a workspace first.</p>
      ) : (
        <div className="ops-grid">
          <section className="panel">
            <div className="panel-head"><h2>Projects ({projects.length})</h2></div>
            {projects.length === 0 ? (
              <p className="muted">No projects yet.</p>
            ) : (
              <table className="bl-table">
                <thead>
                  <tr><th>Name</th><th>Domain</th><th>Every</th><th>Client</th></tr>
                </thead>
                <tbody>
                  {projects.map((p) => (
                    <tr key={p.id}>
                      <td>{p.name}</td>
                      <td className="mono">{p.domain}</td>
                      <td>{Math.round(p.check_frequency_secs / 3600)}h</td>
                      <td>
                        <select
                          value={p.client_id ?? ""}
                          onChange={(e) => void reassign(p.id, e.target.value)}
                          title={clientName(p.client_id)}
                        >
                          <option value="">— none —</option>
                          {clients.map((c) => (
                            <option key={c.id} value={c.id}>{c.name}</option>
                          ))}
                        </select>
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            )}
          </section>

          <section className="panel">
            <div className="panel-head"><h2>New project</h2></div>
            <form onSubmit={create} className="ops-form">
              {error && <div className="auth-error">{error}</div>}
              <label className="auth-field">
                <span>Name</span>
                <input value={name} onChange={(e) => setName(e.target.value)} required />
              </label>
              <label className="auth-field">
                <span>Domain</span>
                <input value={domain} onChange={(e) => setDomain(e.target.value)} placeholder="example.com" required />
              </label>
              <label className="auth-field">
                <span>Client (optional)</span>
                <select value={clientId} onChange={(e) => setClientId(e.target.value)}>
                  <option value="">— none —</option>
                  {clients.map((c) => (
                    <option key={c.id} value={c.id}>{c.name}</option>
                  ))}
                </select>
              </label>
              <label className="auth-field">
                <span>Check every (hours)</span>
                <input type="number" min="1" max="4320" value={hours} onChange={(e) => setHours(e.target.value)} />
              </label>
              <button className="auth-btn" type="submit" disabled={busy}>{busy ? "Creating…" : "Create project"}</button>
            </form>
          </section>
        </div>
      )}
    </main>
  );
}
