"use client";

import { useCallback, useEffect, useState } from "react";
import { useRouter } from "next/navigation";
import { api, orgApi, type ClientView, type MeResponse, type ProjectView } from "@/lib/api";
import { selectedOrgId } from "@/lib/workspace";
import "@/app/auth.css";
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
  const [editId, setEditId] = useState<string | null>(null);
  const [editName, setEditName] = useState("");
  const [editHours, setEditHours] = useState("168");
  const [rowBusy, setRowBusy] = useState<string | null>(null);

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

  function startEdit(p: ProjectView) {
    setEditId(p.id);
    setEditName(p.name);
    setEditHours(String(Math.round(p.check_frequency_secs / 3600)));
    setError(null);
  }

  async function saveEdit() {
    if (!orgId || !editId) return;
    const name = editName.trim();
    const hours = Number(editHours);
    if (!name) {
      setError("Name is required.");
      return;
    }
    if (!Number.isFinite(hours) || hours < 1 || hours > 4320) {
      setError("Check frequency must be between 1 and 4320 hours.");
      return;
    }
    setRowBusy(editId);
    const res = await orgApi.updateProject(orgId, editId, {
      name,
      check_frequency_hours: hours,
    });
    setRowBusy(null);
    if (!res.ok) {
      setError(res.detail);
      return;
    }
    setEditId(null);
    void load(orgId);
  }

  async function remove(p: ProjectView) {
    if (!orgId) return;
    if (
      !confirm(
        `Delete project "${p.name}" and ALL its data (backlinks, campaigns, competitors, discovery seeds)?\nThis cannot be undone.`,
      )
    )
      return;
    setRowBusy(p.id);
    const res = await orgApi.deleteProject(orgId, p.id);
    setRowBusy(null);
    if (!res.ok) {
      setError(res.detail);
      return;
    }
    void load(orgId);
  }

  if (loading) return <div className="auth-wrap"><p style={{ color: "var(--muted)" }}>Loading…</p></div>;
  if (!me) return null;

  return (
    <main className="dash">
      <header className="page-head">
        <h1>Projects</h1>
        <p className="muted">The sites you monitor backlinks for.</p>
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
                  <tr><th>Name</th><th>Domain</th><th>Crawl every</th><th>Client</th><th></th></tr>
                </thead>
                <tbody>
                  {projects.map((p) =>
                    editId === p.id ? (
                      <tr key={p.id}>
                        <td>
                          <input
                            value={editName}
                            onChange={(e) => setEditName(e.target.value)}
                            style={{ width: "100%" }}
                          />
                        </td>
                        <td className="mono muted">{p.domain}</td>
                        <td>
                          <input
                            type="number"
                            min="1"
                            max="4320"
                            value={editHours}
                            onChange={(e) => setEditHours(e.target.value)}
                            style={{ width: 80 }}
                          />{" "}
                          h
                        </td>
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
                        <td style={{ whiteSpace: "nowrap" }}>
                          <button className="btn sm primary" disabled={rowBusy === p.id} onClick={() => void saveEdit()}>
                            {rowBusy === p.id ? "…" : "Save"}
                          </button>{" "}
                          <button className="btn sm" onClick={() => setEditId(null)}>Cancel</button>
                        </td>
                      </tr>
                    ) : (
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
                        <td style={{ whiteSpace: "nowrap" }}>
                          <button className="btn sm" onClick={() => startEdit(p)}>Edit</button>{" "}
                          <button
                            className="btn sm"
                            style={{ color: "#b91c1c", borderColor: "#fecaca" }}
                            disabled={rowBusy === p.id}
                            onClick={() => void remove(p)}
                          >
                            Delete
                          </button>
                        </td>
                      </tr>
                    ),
                  )}
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
