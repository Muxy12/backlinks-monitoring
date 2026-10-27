"use client";
import { PageLoading } from "@/components/PageLoading";

import { useCallback, useEffect, useState } from "react";
import { createPortal } from "react-dom";
import { useRouter } from "next/navigation";
import { api, orgApi, type ClientView, type MeResponse, type ProjectView } from "@/lib/api";
import { selectedOrgId } from "@/lib/workspace";
import "@/app/auth.css";
import "../dashboard/dashboard.css";
import "../operations/operations.css";
import "./projects.css";

// Friendly crawl cadence ("Daily" / "Weekly" / "Every N days") from the stored
// seconds, instead of a raw hour count.
function crawlLabel(secs: number): string {
  const h = Math.round(secs / 3600);
  if (h === 24) return "Daily";
  if (h === 168) return "Weekly";
  if (h % 24 === 0) return `Every ${h / 24} days`;
  return `Every ${h}h`;
}

// Real site favicon as the domain's logo, with a letter-initials fallback when
// it can't be fetched.
function Favicon({ domain, name }: { domain: string; name: string }) {
  const [failed, setFailed] = useState(false);
  const host = domain.replace(/^https?:\/\//, "").replace(/\/.*$/, "");
  if (failed || !host) {
    const initials = name
      .split(/\s+/)
      .slice(0, 2)
      .map((w) => w[0] ?? "")
      .join("")
      .toUpperCase();
    return <span className="dom-fav dom-fav-fallback">{initials || "?"}</span>;
  }
  return (
    // eslint-disable-next-line @next/next/no-img-element
    <img
      className="dom-fav"
      src={`https://www.google.com/s2/favicons?sz=64&domain=${encodeURIComponent(host)}`}
      alt=""
      width={32}
      height={32}
      onError={() => setFailed(true)}
    />
  );
}

export default function ProjectsPage() {
  const router = useRouter();
  const [me, setMe] = useState<MeResponse | null>(null);
  const [orgId, setOrgId] = useState<string | null>(null);
  const [projects, setProjects] = useState<ProjectView[]>([]);
  const [clients, setClients] = useState<ClientView[]>([]);
  const [loading, setLoading] = useState(true);
  const [rowBusy, setRowBusy] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  // One modal for both add and edit. `editId` null ⇒ add.
  const [modalOpen, setModalOpen] = useState(false);
  const [editId, setEditId] = useState<string | null>(null);
  const [fName, setFName] = useState("");
  const [fDomain, setFDomain] = useState("");
  const [fClient, setFClient] = useState("");
  const [fHours, setFHours] = useState("168");
  const [busy, setBusy] = useState(false);
  const [mounted, setMounted] = useState(false);

  useEffect(() => setMounted(true), []);

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

  function openAdd() {
    setEditId(null);
    setFName("");
    setFDomain("");
    setFClient("");
    setFHours("168");
    setError(null);
    setModalOpen(true);
  }

  function openEdit(p: ProjectView) {
    setEditId(p.id);
    setFName(p.name);
    setFDomain(p.domain);
    setFClient(p.client_id ?? "");
    setFHours(String(Math.round(p.check_frequency_secs / 3600)));
    setError(null);
    setModalOpen(true);
  }

  async function submitModal(e: React.FormEvent) {
    e.preventDefault();
    if (!orgId) return;
    const name = fName.trim();
    const hours = Number(fHours);
    if (!name) {
      setError("Name is required.");
      return;
    }
    if (!Number.isFinite(hours) || hours < 1 || hours > 4320) {
      setError("Check frequency must be between 1 and 4320 hours.");
      return;
    }
    setBusy(true);
    const res = editId
      ? await orgApi.updateProject(orgId, editId, { name, check_frequency_hours: hours })
      : await orgApi.createProject(orgId, {
          name,
          domain: fDomain.trim(),
          check_frequency_hours: hours,
          client_id: fClient || undefined,
        });
    setBusy(false);
    if (!res.ok) {
      setError(res.detail);
      return;
    }
    setModalOpen(false);
    void load(orgId);
  }

  async function reassign(projectId: string, newClient: string) {
    if (!orgId) return;
    const res = await orgApi.updateProject(orgId, projectId, {
      client_id: newClient === "" ? null : newClient,
    });
    if (res.ok) void load(orgId);
  }

  async function toggleCrawl(p: ProjectView) {
    if (!orgId) return;
    setRowBusy(p.id);
    const res = await orgApi.updateProject(orgId, p.id, {
      status: p.status === "paused" ? "active" : "paused",
    });
    setRowBusy(null);
    if (!res.ok) {
      setError(res.detail);
      return;
    }
    void load(orgId);
  }

  async function remove(p: ProjectView) {
    if (!orgId) return;
    if (
      !confirm(
        `Delete domain "${p.name}" and ALL its data (backlinks, campaigns, competitors, discovery seeds)?\nThis cannot be undone.`,
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

  if (loading) return <PageLoading />;
  if (!me) return null;

  return (
    <main className="dash">
      <div className="dom-bar">
        <header className="page-head" style={{ marginBottom: 0 }}>
          <h1>Domains</h1>
          <p className="muted">The sites you monitor backlinks for.</p>
        </header>
        {orgId && (
          <button className="btn primary" onClick={openAdd}>
            + Add domain
          </button>
        )}
      </div>

      {error && (
        <div className="auth-error" role="alert" style={{ marginBottom: 12 }}>
          {error}
        </div>
      )}

      {!orgId ? (
        <p className="muted">Create a workspace first.</p>
      ) : (
        <section className="panel">
          <div className="panel-head">
            <h2>Domains ({projects.length})</h2>
          </div>
          {projects.length === 0 ? (
            <p className="muted">No domains yet. Add one to start monitoring its backlinks.</p>
          ) : (
            <div className="bl-scroll">
              <table className="dom-table">
                <thead>
                  <tr>
                    <th>Domain</th>
                    <th>Crawl</th>
                    <th>Client</th>
                    <th></th>
                  </tr>
                </thead>
                <tbody>
                  {projects.map((p) => (
                    <tr key={p.id}>
                      <td>
                        <div className="dom-name">
                          <Favicon domain={p.domain} name={p.name} />
                          <div className="dom-id">
                            <div className="nm">{p.name}</div>
                            <div className="dm" title={p.domain}>
                              {p.domain}
                            </div>
                          </div>
                        </div>
                      </td>
                      <td>
                        <div className="dom-crawl">
                          <button
                            className={`dom-toggle${p.status !== "paused" ? " on" : ""}`}
                            disabled={rowBusy === p.id}
                            onClick={() => void toggleCrawl(p)}
                            aria-label={p.status === "paused" ? "Resume crawl" : "Pause crawl"}
                            title={p.status === "paused" ? "Resume crawl" : "Pause crawl"}
                          />
                          <span className={`dom-crawl-label${p.status === "paused" ? " paused" : ""}`}>
                            {p.status === "paused" ? "Paused" : crawlLabel(p.check_frequency_secs)}
                          </span>
                        </div>
                      </td>
                      <td>
                        <select
                          value={p.client_id ?? ""}
                          onChange={(e) => void reassign(p.id, e.target.value)}
                        >
                          <option value="">- none -</option>
                          {clients.map((c) => (
                            <option key={c.id} value={c.id}>
                              {c.name}
                            </option>
                          ))}
                        </select>
                      </td>
                      <td>
                        <div className="dom-acts">
                          <button className="btn sm" onClick={() => openEdit(p)}>
                            Edit
                          </button>
                          <button
                            className="btn sm danger"
                            disabled={rowBusy === p.id}
                            onClick={() => void remove(p)}
                          >
                            Delete
                          </button>
                        </div>
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          )}
        </section>
      )}

      {mounted &&
        modalOpen &&
        createPortal(
          <div className="dom-modal-backdrop" onClick={() => setModalOpen(false)}>
            <div
              className="dom-modal"
              role="dialog"
              aria-modal="true"
              aria-label={editId ? "Edit domain" : "New domain"}
              onClick={(e) => e.stopPropagation()}
            >
              <h2>{editId ? "Edit domain" : "New domain"}</h2>
              <form onSubmit={submitModal} className="ops-form">
                {error && (
                  <div className="auth-error" role="alert">
                    {error}
                  </div>
                )}
                <label className="auth-field">
                  <span>Name</span>
                  <input value={fName} onChange={(e) => setFName(e.target.value)} autoFocus required />
                </label>
                <label className="auth-field">
                  <span>Domain</span>
                  <input
                    value={fDomain}
                    onChange={(e) => setFDomain(e.target.value)}
                    placeholder="example.com"
                    required
                    disabled={!!editId}
                  />
                </label>
                {!editId && (
                  <label className="auth-field">
                    <span>Client (optional)</span>
                    <select value={fClient} onChange={(e) => setFClient(e.target.value)}>
                      <option value="">- none -</option>
                      {clients.map((c) => (
                        <option key={c.id} value={c.id}>
                          {c.name}
                        </option>
                      ))}
                    </select>
                  </label>
                )}
                <label className="auth-field">
                  <span>Check every (hours)</span>
                  <input
                    type="number"
                    min="1"
                    max="4320"
                    value={fHours}
                    onChange={(e) => setFHours(e.target.value)}
                  />
                </label>
                <div className="dom-modal-actions">
                  <button type="button" className="btn" onClick={() => setModalOpen(false)}>
                    Cancel
                  </button>
                  <button type="submit" className="btn primary" disabled={busy}>
                    {busy ? "Saving…" : editId ? "Save changes" : "Create domain"}
                  </button>
                </div>
              </form>
            </div>
          </div>,
          document.body,
        )}
    </main>
  );
}
