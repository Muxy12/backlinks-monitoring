"use client";

import { useCallback, useEffect, useState, type CSSProperties, type FormEvent } from "react";
import { useRouter } from "next/navigation";
import { api, orgApi, backlinksExportUrl, type BacklinkView, type ClientView, type DashboardResponse, type EventView, type MeResponse, type NotificationsResponse, type ProjectView, type TagView } from "../../lib/api";
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
  const [selected, setSelected] = useState<BacklinkView | null>(null);
  const [events, setEvents] = useState<EventView[] | null>(null);
  const [warranties, setWarranties] = useState<BacklinkView[]>([]);
  const [notifs, setNotifs] = useState<NotificationsResponse | null>(null);
  const [showNotifs, setShowNotifs] = useState(false);
  const [clients, setClients] = useState<ClientView[]>([]);
  const [orgTags, setOrgTags] = useState<TagView[]>([]);
  const [blTags, setBlTags] = useState<TagView[]>([]);
  const [newTag, setNewTag] = useState("");
  const [statusFilter, setStatusFilter] = useState("");
  const [clientFilter, setClientFilter] = useState("");
  const [searchQ, setSearchQ] = useState("");
  const [loading, setLoading] = useState(true);
  const [wsName, setWsName] = useState("");
  const [wsBusy, setWsBusy] = useState(false);
  const [wsErr, setWsErr] = useState<string | null>(null);

  const STATUSES = ["live", "lost", "changed", "nofollow", "redirected", "source_error", "target_error", "blocked", "pending"];

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

  const loadNotifs = useCallback(async (org: string) => {
    const res = await orgApi.notifications(org);
    if (res.ok) setNotifs(res.data);
  }, []);

  const loadOrg = useCallback(
    async (org: string) => {
      const [dash, projs, warr] = await Promise.all([
        orgApi.dashboard(org),
        orgApi.projects(org),
        orgApi.expiringWarranties(org),
      ]);
      if (dash.ok) setSummary(dash.data);
      if (projs.ok) {
        setProjects(projs.data);
        const p = projs.data[0]?.id ?? null;
        setProjectId(p);
      }
      if (warr.ok) setWarranties(warr.data);
      const [tags, cls] = await Promise.all([orgApi.tags(org), orgApi.clients(org)]);
      if (tags.ok) setOrgTags(tags.data);
      if (cls.ok) setClients(cls.data);
      await loadNotifs(org);
    },
    [loadNotifs],
  );

  async function markRead(id: string) {
    if (!orgId) return;
    const res = await orgApi.markNotificationRead(orgId, id);
    if (res.ok) await loadNotifs(orgId);
  }

  useEffect(() => {
    if (orgId) void loadOrg(orgId);
  }, [orgId, loadOrg]);

  useEffect(() => {
    // A client filter shows that client's backlinks across all its projects;
    // otherwise the view is scoped to the selected project.
    if (!orgId || (!clientFilter && !projectId)) {
      setBacklinks([]);
      return;
    }
    const t = setTimeout(() => {
      orgApi
        .searchBacklinks(orgId, {
          projectId: clientFilter ? undefined : projectId || undefined,
          clientId: clientFilter || undefined,
          status: statusFilter || undefined,
          q: searchQ.trim() || undefined,
        })
        .then((res) => {
          if (res.ok) setBacklinks(res.data);
        });
    }, 200); // debounce the search box
    return () => clearTimeout(t);
  }, [orgId, projectId, clientFilter, statusFilter, searchQ]);

  async function openDetail(b: BacklinkView) {
    setSelected(b);
    setEvents(null);
    setBlTags([]);
    setNewTag("");
    if (orgId) {
      const [ev, tg] = await Promise.all([
        orgApi.backlinkEvents(orgId, b.id),
        orgApi.backlinkTags(orgId, b.id),
      ]);
      setEvents(ev.ok ? ev.data : []);
      if (tg.ok) setBlTags(tg.data);
    }
  }

  async function refreshBlTags(id: string) {
    if (!orgId) return;
    const tg = await orgApi.backlinkTags(orgId, id);
    if (tg.ok) setBlTags(tg.data);
  }

  async function attachExisting(tagId: string) {
    if (!orgId || !selected || !tagId) return;
    const res = await orgApi.attachTag(orgId, selected.id, tagId);
    if (res.ok) await refreshBlTags(selected.id);
  }

  async function removeTag(tagId: string) {
    if (!orgId || !selected) return;
    const res = await orgApi.detachTag(orgId, selected.id, tagId);
    if (res.ok) await refreshBlTags(selected.id);
  }

  async function createAndAttach() {
    if (!orgId || !selected) return;
    const name = newTag.trim();
    if (!name) return;
    const created = await orgApi.createTag(orgId, name);
    if (created.ok) {
      await orgApi.attachTag(orgId, selected.id, created.data.id);
      setNewTag("");
      const [tags, blt] = await Promise.all([orgApi.tags(orgId), orgApi.backlinkTags(orgId, selected.id)]);
      if (tags.ok) setOrgTags(tags.data);
      if (blt.ok) setBlTags(blt.data);
    }
  }

  async function archiveSelected() {
    if (!orgId || !selected || !projectId) return;
    const res = await orgApi.archiveBacklink(orgId, selected.id);
    if (res.ok) {
      setSelected(null);
      const list = await orgApi.backlinks(orgId, projectId);
      if (list.ok) setBacklinks(list.data);
    }
  }

  if (loading) return <div className="auth-wrap"><p style={{ color: "var(--muted)" }}>Loading…</p></div>;
  if (!me) return null;

  const money = (rows: { currency: string; amount: string }[]) =>
    rows.length === 0 ? "—" : rows.map((r) => `${r.amount} ${r.currency}`).join(" · ");

  // The client these backlinks belong to: the explicit client filter, else the
  // selected project's client.
  const selectedProject = projects.find((p) => p.id === projectId) ?? null;
  const activeClientId = clientFilter || selectedProject?.client_id || "";
  const selectedClient = activeClientId
    ? clients.find((c) => c.id === activeClientId) ?? null
    : null;

  async function createWorkspace(e: FormEvent) {
    e.preventDefault();
    const name = wsName.trim();
    if (name.length < 2) {
      setWsErr("Enter a workspace name (at least 2 characters).");
      return;
    }
    setWsErr(null);
    setWsBusy(true);
    const res = await api.createOrganization(name);
    if (!res.ok) {
      setWsBusy(false);
      setWsErr(res.detail || "Could not create the workspace.");
      return;
    }
    // Reload the session so the new org (and its id) propagate.
    const meRes = await api.me();
    setWsBusy(false);
    if (meRes.ok) {
      setMe(meRes.data);
      setOrgId(meRes.data.organizations[0]?.id ?? null);
      setWsName("");
    }
  }

  // Segments for the status-breakdown bar (only statuses that actually occur).
  const distSegments = STATUSES.map((st) => ({ st, n: summary?.counts[st] ?? 0 })).filter((s) => s.n > 0);
  const distTotal = distSegments.reduce((acc, s) => acc + s.n, 0);

  return (
    <main className="dash">
      <header className="dash-head">
        <div className="dash-id">
          <div className="brand"><span className="brand-mark" />Backlio</div>
          <h1>
            {me.organizations[0]?.name ?? "Dashboard"}
            {me.organizations[0]?.role && <span className="role-pill">{me.organizations[0].role}</span>}
          </h1>
          <p className="muted">{me.user.email}</p>
        </div>
        <nav className="head-actions">
          <a className="btn" href="/projects">Projects</a>
          <a className="btn" href="/clients">Clients</a>
          <a className="btn" href="/operations">Operations</a>
          <a className="btn" href="/import">Import</a>
          <a className="btn" href="/team">Team</a>
          <button className="btn bell" onClick={() => setShowNotifs((v) => !v)} aria-label="Notifications">
            🔔
            {notifs && notifs.unread > 0 && <span className="bell-badge">{notifs.unread}</span>}
          </button>
          <button className="btn" onClick={async () => { await api.logout(); router.push("/login"); }}>Log out</button>
        </nav>
      </header>

      {me.organizations.length === 0 ? (
        <section className="panel create-ws">
          <div className="empty-icon">🚀</div>
          <h2>Create your workspace</h2>
          <p className="muted">A workspace holds your clients, projects and backlinks. Name it after your agency or your own site.</p>
          <form onSubmit={createWorkspace} className="ws-form">
            <input
              type="text"
              placeholder="e.g. Acme SEO"
              value={wsName}
              onChange={(e) => setWsName(e.target.value)}
              disabled={wsBusy}
              autoFocus
            />
            <button type="submit" className="btn primary" disabled={wsBusy}>
              {wsBusy ? "Creating…" : "Create workspace"}
            </button>
          </form>
          {wsErr && <p className="error">{wsErr}</p>}
        </section>
      ) : (
        <>
          <section className="tiles">
            <Tile label="Monitored" value={summary?.monitored ?? 0} icon="📡" hint="Active backlinks" />
            <Tile label="Live" value={summary?.counts["live"] ?? 0} accent="var(--green)" icon="✓" hint="Confirmed present" />
            <Tile label="Needs attention" value={summary?.needs_attention ?? 0} accent="var(--yellow)" icon="!" hint="Lost, changed or errored" />
            <Tile label="Value at risk" value={money(summary?.value_at_risk ?? [])} accent="var(--red)" icon="⚠" hint="Cost of at-risk links" />
          </section>

          {distTotal > 0 && (
            <section className="dist">
              <div className="dist-head">
                <h2>Status breakdown</h2>
                <span className="dist-total">{distTotal} link{distTotal > 1 ? "s" : ""}</span>
              </div>
              <div className="dist-bar">
                {distSegments.map((s) => (
                  <div
                    key={s.st}
                    className="dist-seg"
                    style={{ flexGrow: s.n, background: statusColor(s.st) }}
                    title={`${statusLabel(s.st)}: ${s.n}`}
                  />
                ))}
              </div>
              <div className="legend">
                {distSegments.map((s) => (
                  <span key={s.st} className="legend-item" style={{ color: statusColor(s.st) }}>
                    <span className="legend-dot" />
                    <span style={{ color: "var(--muted)" }}>{statusLabel(s.st)}</span>
                    <b>{s.n}</b>
                  </span>
                ))}
              </div>
            </section>
          )}

          <section className="panel">
            <div className="panel-head">
              <h2>
                Backlinks
                {selectedClient && <span className="client-badge">{selectedClient.name}</span>}
              </h2>
              <div className="panel-actions">
                {clients.length > 0 && (
                  <select value={clientFilter} onChange={(e) => setClientFilter(e.target.value)} title="Filter by client">
                    <option value="">All clients</option>
                    {clients.map((c) => (
                      <option key={c.id} value={c.id}>{c.name}</option>
                    ))}
                  </select>
                )}
                {projects.length > 0 && (
                  <select
                    value={projectId ?? ""}
                    onChange={(e) => setProjectId(e.target.value)}
                    disabled={!!clientFilter}
                    title={clientFilter ? "Clear the client filter to pick a project" : "Project"}
                  >
                    {projects.map((p) => (
                      <option key={p.id} value={p.id}>{p.name} — {p.domain}</option>
                    ))}
                  </select>
                )}
                <input
                  className="bl-search"
                  placeholder="Search domain/anchor"
                  value={searchQ}
                  onChange={(e) => setSearchQ(e.target.value)}
                />
                <select value={statusFilter} onChange={(e) => setStatusFilter(e.target.value)}>
                  <option value="">All statuses</option>
                  {STATUSES.map((st) => (
                    <option key={st} value={st}>{statusLabel(st)}</option>
                  ))}
                </select>
                <a className="btn sm primary" href="/import">+ Add backlinks</a>
                {orgId && (
                  <a className="btn sm" href={backlinksExportUrl(orgId, projectId ?? undefined, clientFilter || undefined)}>
                    Export CSV
                  </a>
                )}
              </div>
            </div>
            {warranties.length > 0 && (
              <div className="warranty-note">
                ⚠ {warranties.length} backlink{warranties.length > 1 ? "s" : ""} with a warranty
                expiring soon or lapsed:{" "}
                {warranties
                  .slice(0, 5)
                  .map((w) => `${w.source_domain} (${w.warranty_status ?? "?"})`)
                  .join(", ")}
                {warranties.length > 5 ? "…" : ""}
              </div>
            )}
            {backlinks.length === 0 ? (
              <div className="empty">
                <div className="empty-icon">🔗</div>
                <h3>{projectId || clientFilter ? "No backlinks here yet" : "Pick a project to view its backlinks"}</h3>
                <p>
                  {projects.length === 0
                    ? "Create a project first, then import the backlinks you want to monitor."
                    : "Import a CSV of backlinks and Backlio will track their status, anchors, rel and warranties."}
                </p>
                <div className="empty-cta">
                  {projects.length === 0 ? (
                    <a className="btn primary" href="/projects">Create a project</a>
                  ) : (
                    <a className="btn primary" href="/import">Import backlinks</a>
                  )}
                  <a className="btn" href="/projects">Manage projects</a>
                </div>
              </div>
            ) : (
              <table className="bl-table">
                <thead>
                  <tr><th>Status</th><th>Source</th><th>Target</th><th>Anchor</th><th>Rel</th><th>Cost</th></tr>
                </thead>
                <tbody>
                  {backlinks.map((b) => (
                    <tr key={b.id} className="row-click" onClick={() => void openDetail(b)}>
                      <td><span className="badge" style={{ "--st": statusColor(b.status) } as CSSProperties}><span className="dot" />{statusLabel(b.status)}</span></td>
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
      {showNotifs && (
        <div className="drawer-backdrop" onClick={() => setShowNotifs(false)}>
          <aside className="drawer" onClick={(e) => e.stopPropagation()}>
            <div className="panel-head">
              <h2>Notifications</h2>
              <div className="panel-actions"><a className="btn sm" href="/settings/notifications">Settings</a><button className="btn" onClick={() => setShowNotifs(false)}>Close</button></div>
            </div>
            {!notifs || notifs.items.length === 0 ? (
              <p className="muted">No notifications yet.</p>
            ) : (
              <ul className="notif-list">
                {notifs.items.map((n) => (
                  <li key={n.id} className={n.read ? "notif read" : "notif"}>
                    <div>
                      <div className="notif-title">{n.title}</div>
                      {n.body && <div className="muted">{n.body}</div>}
                      <div className="muted mono">{new Date(n.created_at).toLocaleString()}</div>
                    </div>
                    {!n.read && (
                      <button className="btn sm" onClick={() => void markRead(n.id)}>Mark read</button>
                    )}
                  </li>
                ))}
              </ul>
            )}
          </aside>
        </div>
      )}
      {selected && (
        <div className="drawer-backdrop" onClick={() => setSelected(null)}>
          <aside className="drawer" onClick={(e) => e.stopPropagation()}>
            <div className="panel-head">
              <h2>Backlink history</h2>
              <div className="panel-actions">
                <button className="btn sm" onClick={() => void archiveSelected()}>Archive</button>
                <button className="btn" onClick={() => setSelected(null)}>Close</button>
              </div>
            </div>
            <p className="muted mono">{selected.source_url}</p>
            <p className="muted mono">→ {selected.target_url}</p>
            <p style={{ margin: "10px 0" }}>
              <span className="badge" style={{ "--st": statusColor(selected.status) } as CSSProperties}><span className="dot" />{statusLabel(selected.status)}</span>
            </p>
            <div className="tags-row">
              {blTags.map((t) => (
                <span key={t.id} className="tag-chip" style={t.color ? { borderColor: t.color, color: t.color } : undefined}>
                  {t.name}
                  <button className="tag-x" onClick={() => void removeTag(t.id)} aria-label={`Remove ${t.name}`}>×</button>
                </span>
              ))}
              {blTags.length === 0 && <span className="muted">No tags</span>}
            </div>
            <div className="tags-add">
              <select
                value=""
                onChange={(e) => { if (e.target.value) void attachExisting(e.target.value); }}
              >
                <option value="">Add tag…</option>
                {orgTags
                  .filter((t) => !blTags.some((b) => b.id === t.id))
                  .map((t) => (
                    <option key={t.id} value={t.id}>{t.name}</option>
                  ))}
              </select>
              <input
                placeholder="new tag"
                value={newTag}
                onChange={(e) => setNewTag(e.target.value)}
                onKeyDown={(e) => { if (e.key === "Enter") void createAndAttach(); }}
              />
              <button className="btn sm" onClick={() => void createAndAttach()}>Add</button>
            </div>
            {events === null ? (
              <p className="muted">Loading history…</p>
            ) : events.length === 0 ? (
              <p className="muted">No changes recorded yet.</p>
            ) : (
              <ul className="timeline">
                {events.map((e, i) => (
                  <li key={i}>
                    <span className="tl-dot" />
                    <div>
                      <div className="tl-type">{statusLabel(e.event_type)}</div>
                      <div className="muted mono">{new Date(e.detected_at).toLocaleString()}</div>
                    </div>
                  </li>
                ))}
              </ul>
            )}
          </aside>
        </div>
      )}
    </main>
  );
}

function Tile({
  label,
  value,
  accent,
  icon,
  hint,
}: {
  label: string;
  value: number | string;
  accent?: string;
  icon?: string;
  hint?: string;
}) {
  return (
    <div className="tile" style={accent ? ({ "--tile-accent": accent } as CSSProperties) : undefined}>
      <div className="tile-top">
        <div className="tile-label">{label}</div>
        {icon && <div className="tile-icon">{icon}</div>}
      </div>
      <div className="tile-value">{value}</div>
      {hint && <div className="tile-hint">{hint}</div>}
    </div>
  );
}
