"use client";
import { PageLoading } from "@/components/PageLoading";

import { useCallback, useEffect, useState } from "react";
import { createPortal } from "react-dom";
import { useRouter } from "next/navigation";
import {
  api,
  orgApi,
  type ClientView,
  type MeResponse,
  type ProjectView,
  type SiteAnalysis,
  type PositionedKeyword,
} from "@/lib/api";
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
  // Site-analysis drawer.
  const [anProject, setAnProject] = useState<ProjectView | null>(null);
  const [an, setAn] = useState<SiteAnalysis | null>(null);
  const [anPos, setAnPos] = useState<PositionedKeyword[] | null>(null);
  const [anBusy, setAnBusy] = useState(false);
  const [anForm, setAnForm] = useState({
    theme: "",
    sub_themes: "",
    products: "",
    keywords: "",
    summary: "",
  });

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

  const loadAnalysis = useCallback(async (org: string, projectId: string) => {
    const res = await orgApi.siteAnalysis(org, projectId);
    if (!res.ok) return;
    setAn(res.data);
    if (res.data) {
      setAnForm({
        theme: res.data.theme ?? "",
        sub_themes: res.data.sub_themes.join(", "),
        products: res.data.products.join(", "),
        keywords: res.data.keywords.join(", "),
        summary: res.data.summary ?? "",
      });
    }
  }, []);

  function openAnalysis(p: ProjectView) {
    setAnProject(p);
    setAn(null);
    setAnPos(null);
    setAnForm({ theme: "", sub_themes: "", products: "", keywords: "", summary: "" });
    if (orgId) {
      void loadAnalysis(orgId, p.id);
      void orgApi.positionedKeywords(orgId, p.id).then((r) => setAnPos(r.ok ? r.data : []));
    }
  }

  async function runAnalysis() {
    if (!orgId || !anProject) return;
    setAnBusy(true);
    const res = await orgApi.analyzeSite(orgId, anProject.id);
    setAnBusy(false);
    if (res.ok) setAn(res.data);
  }

  async function saveAnalysis() {
    if (!orgId || !anProject) return;
    const toList = (s: string) =>
      s
        .split(",")
        .map((x) => x.trim())
        .filter(Boolean);
    setAnBusy(true);
    const res = await orgApi.updateSiteAnalysis(orgId, anProject.id, {
      theme: anForm.theme.trim() || null,
      sub_themes: toList(anForm.sub_themes),
      products: toList(anForm.products),
      keywords: toList(anForm.keywords),
      summary: anForm.summary.trim() || null,
    });
    setAnBusy(false);
    if (res.ok) setAn(res.data);
  }

  useEffect(() => {
    if (!orgId || !anProject) return;
    if (an?.status !== "queued" && an?.status !== "running") return;
    const t = setInterval(() => void loadAnalysis(orgId, anProject.id), 3000);
    return () => clearInterval(t);
  }, [orgId, anProject, an?.status, loadAnalysis]);

  if (loading) return <PageLoading />;
  if (!me) return null;

  const anRunning = an?.status === "queued" || an?.status === "running";

  return (
    <main className="dash">
      <div className="dom-bar">
        <header className="page-head" style={{ marginBottom: 0 }}>
          <h1>My sites</h1>
          <p className="muted">Your sites — monitored, and analysed for link-building topics.</p>
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
                          <button className="btn sm" onClick={() => openAnalysis(p)}>
                            Analysis
                          </button>
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

      {anProject && (
        <div
          onClick={() => setAnProject(null)}
          style={{ position: "fixed", inset: 0, background: "rgba(5,10,25,.45)", zIndex: 60, display: "flex", justifyContent: "flex-end" }}
        >
          <aside
            role="dialog"
            aria-modal="true"
            aria-label={`Analysis for ${anProject.domain}`}
            onClick={(e) => e.stopPropagation()}
            style={{ width: "min(560px, 100%)", height: "100%", overflowY: "auto", background: "var(--surface)", borderLeft: "1px solid var(--border)", padding: 22 }}
          >
            <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", gap: 8 }}>
              <h2 className="mono" style={{ margin: 0, fontSize: 18 }}>{anProject.domain}</h2>
              <button className="btn sm" onClick={() => setAnProject(null)}>Close</button>
            </div>
            <p className="muted" style={{ marginTop: 6 }}>AI analysis of your site's topics — editable.</p>

            <div style={{ display: "flex", gap: 10, alignItems: "center", marginTop: 12, flexWrap: "wrap" }}>
              <button className="btn primary" disabled={anBusy || anRunning} onClick={() => void runAnalysis()}>
                {anRunning ? "Analyzing…" : an && an.status === "done" ? "Re-analyze" : "Analyze"}
              </button>
              {an?.source && (
                <span style={{ padding: "2px 10px", borderRadius: 999, fontSize: 12, fontWeight: 700, background: an.source === "ai" ? "var(--primary-weak)" : "var(--surface-2)", color: an.source === "ai" ? "var(--primary-press)" : "var(--muted)" }}>
                  {an.source === "ai" ? "AI" : "Heuristic"}
                </span>
              )}
              {an?.language && <span className="muted">· {an.language}</span>}
              {an?.edited && <span className="muted">· edited</span>}
            </div>

            {anRunning && <p className="muted" style={{ marginTop: 12 }}>Reading your site…</p>}
            {an?.status === "error" && (
              <div className="auth-error" role="alert" style={{ marginTop: 12 }}>{an.error ?? "Analysis failed."}</div>
            )}
            {!an && !anRunning && (
              <p className="muted" style={{ marginTop: 12 }}>Not analysed yet — click Analyze to read your site's topics.</p>
            )}

            {an && an.status === "done" && (
              <div style={{ marginTop: 16, display: "grid", gap: 12 }}>
                {[
                  ["Theme", "theme", false],
                  ["Sub-themes (comma-separated)", "sub_themes", false],
                  ["Products / services (comma-separated)", "products", false],
                  ["Detected keywords (comma-separated)", "keywords", false],
                  ["Summary", "summary", true],
                ].map(([label, key, multi]) => (
                  <label key={key as string} style={{ display: "grid", gap: 4, fontSize: 12, color: "var(--muted)" }}>
                    {label as string}
                    {multi ? (
                      <textarea
                        rows={3}
                        value={anForm[key as keyof typeof anForm]}
                        onChange={(e) => setAnForm((f) => ({ ...f, [key as string]: e.target.value }))}
                        style={{ padding: "8px 10px", borderRadius: 8, border: "1px solid var(--border)", background: "var(--bg)", color: "var(--fg)", font: "inherit" }}
                      />
                    ) : (
                      <input
                        value={anForm[key as keyof typeof anForm]}
                        onChange={(e) => setAnForm((f) => ({ ...f, [key as string]: e.target.value }))}
                        style={{ padding: "8px 10px", borderRadius: 8, border: "1px solid var(--border)", background: "var(--bg)", color: "var(--fg)" }}
                      />
                    )}
                  </label>
                ))}
                <div>
                  <button className="btn primary" disabled={anBusy} onClick={() => void saveAnalysis()}>
                    {anBusy ? "Saving…" : "Save"}
                  </button>
                </div>
                <p className="muted" style={{ fontSize: 11 }}>
                  Detected topics — not verified Google positions.
                </p>
              </div>
            )}

            <div style={{ marginTop: 18, borderTop: "1px solid var(--border)", paddingTop: 14 }}>
              <div style={{ fontWeight: 700, marginBottom: 6 }}>
                Positioned keywords{" "}
                <span className="muted" style={{ fontWeight: 400, fontSize: 12 }}>
                  · real Google positions (GSC)
                </span>
              </div>
              {anPos == null ? (
                <p className="muted" style={{ fontSize: 13 }}>
                  Loading…
                </p>
              ) : anPos.length === 0 ? (
                <p className="muted" style={{ fontSize: 13 }}>
                  No Search Console data. Connect Google Search Console in Settings → Integrations to
                  see the keywords your site really ranks for.
                </p>
              ) : (
                <table className="dom-table" style={{ marginTop: 6 }}>
                  <thead>
                    <tr>
                      <th>Keyword</th>
                      <th>Pos</th>
                      <th>Impr.</th>
                      <th>Clicks</th>
                    </tr>
                  </thead>
                  <tbody>
                    {anPos.map((k, i) => (
                      <tr key={i}>
                        <td>{k.query}</td>
                        <td className="mono">{k.position}</td>
                        <td className="mono">{k.impressions.toLocaleString()}</td>
                        <td className="mono">{k.clicks.toLocaleString()}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              )}
            </div>
          </aside>
        </div>
      )}
    </main>
  );
}
