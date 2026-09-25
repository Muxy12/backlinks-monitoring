"use client";

import { useCallback, useEffect, useState, type CSSProperties, type FormEvent } from "react";
import { useRouter } from "next/navigation";
import { api, orgApi, backlinksExportUrl, type BacklinkView, type ClientView, type DashboardResponse, type EventsSummary, type EventView, type MeResponse, type NotificationsResponse, type OccurrenceView, type ProjectView, type TagView } from "@/lib/api";
import { statusColor, statusLabel } from "@/lib/status";
import "@/app/auth.css";
import "./dashboard.css";

export default function DashboardPage() {
  const router = useRouter();
  const [me, setMe] = useState<MeResponse | null>(null);
  const [orgId, setOrgId] = useState<string | null>(null);
  const [summary, setSummary] = useState<DashboardResponse | null>(null);
  const [evsum, setEvsum] = useState<EventsSummary | null>(null);
  const [projects, setProjects] = useState<ProjectView[]>([]);
  const [projectId, setProjectId] = useState<string | null>(null);
  const [backlinks, setBacklinks] = useState<BacklinkView[]>([]);
  const [selected, setSelected] = useState<BacklinkView | null>(null);
  const [events, setEvents] = useState<EventView[] | null>(null);
  const [occurrences, setOccurrences] = useState<OccurrenceView[]>([]);
  const [tier2, setTier2] = useState<BacklinkView[]>([]);
  const [t2csv, setT2csv] = useState("");
  const [t2msg, setT2msg] = useState<string | null>(null);
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
  const [showAdd, setShowAdd] = useState(false);
  const [addBusy, setAddBusy] = useState(false);
  const [addErr, setAddErr] = useState<string | null>(null);
  const [addForm, setAddForm] = useState({
    project_id: "",
    source_url: "",
    target_url: "",
    expected_rel_type: "dofollow",
    expected_anchor_text: "",
    cost: "",
    currency: "",
  });

  const STATUSES = ["live", "suspected_lost", "lost", "changed", "nofollow", "redirected", "source_error", "target_error", "blocked", "pending"];

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
      const [dash, projs, warr, evs] = await Promise.all([
        orgApi.dashboard(org),
        orgApi.projects(org),
        orgApi.expiringWarranties(org),
        orgApi.eventsSummary(org, 30),
      ]);
      if (evs.ok) setEvsum(evs.data);
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
    setOccurrences([]);
    setTier2([]);
    setT2csv("");
    setT2msg(null);
    setNewTag("");
    if (orgId) {
      const [ev, tg, oc, t2] = await Promise.all([
        orgApi.backlinkEvents(orgId, b.id),
        orgApi.backlinkTags(orgId, b.id),
        orgApi.backlinkOccurrences(orgId, b.id),
        orgApi.backlinkTier2(orgId, b.id),
      ]);
      setEvents(ev.ok ? ev.data : []);
      if (tg.ok) setBlTags(tg.data);
      if (oc.ok) setOccurrences(oc.data);
      if (t2.ok) setTier2(t2.data);
    }
  }

  async function importTier2() {
    if (!orgId || !selected) return;
    const rows = t2csv
      .split(/\r?\n/)
      .map((l) => l.trim())
      .filter(Boolean)
      .map((l) => {
        const [source_url, anchor, rel] = l.split(",").map((s) => s?.trim());
        return { source_url, anchor, rel };
      })
      .filter((r) => r.source_url);
    if (rows.length === 0) {
      setT2msg("Paste rows: source_url,anchor,rel");
      return;
    }
    const res = await orgApi.importTier2(orgId, selected.id, rows);
    if (!res.ok) {
      setT2msg(res.detail || "Import failed.");
      return;
    }
    setT2msg(`Imported ${res.data.imported} (${res.data.new} new).`);
    setT2csv("");
    const t2 = await orgApi.backlinkTier2(orgId, selected.id);
    if (t2.ok) setTier2(t2.data);
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

  function openAdd() {
    setAddErr(null);
    setAddForm((f) => ({ ...f, project_id: projectId ?? projects[0]?.id ?? "" }));
    setShowAdd(true);
  }

  async function submitBacklink(e: FormEvent) {
    e.preventDefault();
    if (!orgId) return;
    const f = addForm;
    if (!f.project_id) {
      setAddErr("Pick a project for this backlink.");
      return;
    }
    const src = f.source_url.trim();
    const tgt = f.target_url.trim();
    if (!/^https?:\/\//.test(src) || !/^https?:\/\//.test(tgt)) {
      setAddErr("Source and target must start with http:// or https://.");
      return;
    }
    setAddErr(null);
    setAddBusy(true);
    const res = await orgApi.createBacklink(orgId, {
      project_id: f.project_id,
      source_url: src,
      target_url: tgt,
      expected_rel_type: f.expected_rel_type,
      ...(f.expected_anchor_text.trim() ? { expected_anchor_text: f.expected_anchor_text.trim() } : {}),
      ...(f.cost.trim() ? { cost: Number(f.cost), currency: f.currency.trim() || "USD" } : {}),
    });
    setAddBusy(false);
    if (!res.ok) {
      setAddErr(res.detail || "Could not add the backlink.");
      return;
    }
    setShowAdd(false);
    setAddForm({ project_id: f.project_id, source_url: "", target_url: "", expected_rel_type: "dofollow", expected_anchor_text: "", cost: "", currency: "" });
    // Show the project we just added to, and refresh the numbers + list.
    setClientFilter("");
    setProjectId(f.project_id);
    const [dash, list] = await Promise.all([
      orgApi.dashboard(orgId),
      orgApi.searchBacklinks(orgId, { projectId: f.project_id }),
    ]);
    if (dash.ok) setSummary(dash.data);
    if (list.ok) setBacklinks(list.data);
  }

  // Segments for the status-breakdown bar (only statuses that actually occur).
  const distSegments = STATUSES.map((st) => ({ st, n: summary?.counts[st] ?? 0 })).filter((s) => s.n > 0);
  const distTotal = distSegments.reduce((acc, s) => acc + s.n, 0);

  return (
    <main className="dash">
      <header className="dash-head">
        <div className="dash-id">
          <h1>
            {me.organizations[0]?.name ?? "Dashboard"}
            {me.organizations[0]?.role && <span className="role-pill">{me.organizations[0].role}</span>}
          </h1>
          <p className="muted">{me.user.email}</p>
        </div>
        <div className="head-actions">
          <button className="btn bell" onClick={() => setShowNotifs((v) => !v)} aria-label="Notifications">
            🔔
            {notifs && notifs.unread > 0 && <span className="bell-badge">{notifs.unread}</span>}
          </button>
        </div>
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

          {summary && (
            <div className="substats">
              <span><b>{summary.referring_domains}</b> referring domains</span>
              <span><b>{summary.referring_pages}</b> referring pages</span>
              <span><b>{summary.occurrences}</b> occurrences</span>
            </div>
          )}

          {evsum && Object.keys(evsum.counts).length > 0 && (
            <div className="substats evo">
              <span className="evo-label">Last {evsum.days} days:</span>
              {evsum.counts["backlink_found"] ? <span><b>+{evsum.counts["backlink_found"]}</b> found</span> : null}
              {evsum.counts["backlink_lost"] ? <span><b>{evsum.counts["backlink_lost"]}</b> lost</span> : null}
              {evsum.counts["backlink_restored"] ? <span><b>{evsum.counts["backlink_restored"]}</b> recovered</span> : null}
              {evsum.counts["dofollow_to_nofollow"] ? <span><b>{evsum.counts["dofollow_to_nofollow"]}</b> → nofollow</span> : null}
              {evsum.counts["anchor_changed"] ? <span><b>{evsum.counts["anchor_changed"]}</b> anchor changes</span> : null}
              {evsum.counts["target_changed"] ? <span><b>{evsum.counts["target_changed"]}</b> target changes</span> : null}
              {evsum.counts["link_moved"] ? <span><b>{evsum.counts["link_moved"]}</b> moved</span> : null}
            </div>
          )}

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
                {projects.length > 0 && (
                  <button className="btn sm primary" onClick={openAdd}>+ Add backlink</button>
                )}
                <a className="btn sm" href="/import">Import</a>
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
                    <>
                      <a className="btn primary" href="/projects">Create a project</a>
                      <a className="btn" href="/projects">Manage projects</a>
                    </>
                  ) : (
                    <>
                      <button className="btn primary" onClick={openAdd}>+ Add backlink</button>
                      <a className="btn" href="/import">Import CSV</a>
                    </>
                  )}
                </div>
              </div>
            ) : (
              <table className="bl-table">
                <thead>
                  <tr>
                    <th>Status</th>
                    <th>Source</th>
                    <th>Target</th>
                    <th>Anchor</th>
                    <th>Rel</th>
                    <th>HTTP</th>
                    <th>Checked</th>
                  </tr>
                </thead>
                <tbody>
                  {backlinks.map((b) => (
                    <tr key={b.id} className="row-click" onClick={() => void openDetail(b)}>
                      <td><span className="badge" style={{ "--st": statusColor(b.status) } as CSSProperties}><span className="dot" />{statusLabel(b.status)}</span></td>
                      <td className="mono" title={b.source_url}>{b.source_domain}</td>
                      <td className="mono" title={b.target_url}>{b.target_domain}</td>
                      <td className="cell-anchor" title={b.detected_anchor_text ?? b.expected_anchor_text ?? ""}>{b.detected_anchor_text ?? b.expected_anchor_text ?? "—"}</td>
                      <td><RelChips b={b} /></td>
                      <td className="mono">{httpPair(b)}</td>
                      <td className="mono">{fmtShort(b.last_checked_at)}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            )}
          </section>
        </>
      )}
      {showAdd && (
        <div className="drawer-backdrop" onClick={() => setShowAdd(false)}>
          <aside className="drawer" onClick={(e) => e.stopPropagation()}>
            <div className="panel-head">
              <h2>Add a backlink</h2>
              <button className="btn" onClick={() => setShowAdd(false)}>Close</button>
            </div>
            <form onSubmit={submitBacklink} className="add-form">
              <label>
                <span>Project</span>
                <select
                  value={addForm.project_id}
                  onChange={(e) => setAddForm((f) => ({ ...f, project_id: e.target.value }))}
                >
                  {projects.map((p) => (
                    <option key={p.id} value={p.id}>{p.name} — {p.domain}</option>
                  ))}
                </select>
              </label>
              <label>
                <span>Source URL (the page that links to you)</span>
                <input
                  placeholder="https://blog.example.com/post"
                  value={addForm.source_url}
                  onChange={(e) => setAddForm((f) => ({ ...f, source_url: e.target.value }))}
                  autoFocus
                />
              </label>
              <label>
                <span>Target URL (your page)</span>
                <input
                  placeholder="https://yourdomain.com/landing"
                  value={addForm.target_url}
                  onChange={(e) => setAddForm((f) => ({ ...f, target_url: e.target.value }))}
                />
              </label>
              <div className="add-row">
                <label>
                  <span>Rel</span>
                  <select
                    value={addForm.expected_rel_type}
                    onChange={(e) => setAddForm((f) => ({ ...f, expected_rel_type: e.target.value }))}
                  >
                    <option value="dofollow">dofollow</option>
                    <option value="nofollow">nofollow</option>
                    <option value="sponsored">sponsored</option>
                    <option value="ugc">ugc</option>
                  </select>
                </label>
                <label>
                  <span>Anchor text (optional)</span>
                  <input
                    placeholder="e.g. best SEO tools"
                    value={addForm.expected_anchor_text}
                    onChange={(e) => setAddForm((f) => ({ ...f, expected_anchor_text: e.target.value }))}
                  />
                </label>
              </div>
              <div className="add-row">
                <label>
                  <span>Cost (optional)</span>
                  <input
                    type="number"
                    min="0"
                    step="0.01"
                    placeholder="0.00"
                    value={addForm.cost}
                    onChange={(e) => setAddForm((f) => ({ ...f, cost: e.target.value }))}
                  />
                </label>
                <label>
                  <span>Currency</span>
                  <input
                    placeholder="USD"
                    value={addForm.currency}
                    onChange={(e) => setAddForm((f) => ({ ...f, currency: e.target.value }))}
                  />
                </label>
              </div>
              {addErr && <p className="error">{addErr}</p>}
              <button className="btn primary" type="submit" disabled={addBusy}>
                {addBusy ? "Adding…" : "Add backlink"}
              </button>
            </form>
          </aside>
        </div>
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
            <p style={{ margin: "6px 0 14px" }}>
              <span className="badge" style={{ "--st": statusColor(selected.status) } as CSSProperties}><span className="dot" />{statusLabel(selected.status)}</span>
            </p>
            <dl className="detail-grid">
              <div><dt>Source</dt><dd className="mono br">{selected.source_url}</dd></div>
              <div><dt>Target</dt><dd className="mono br">{selected.target_url}</dd></div>
              <div><dt>Target final URL</dt><dd className="mono br">{selected.target_final_url ?? "—"}</dd></div>
              <div><dt>Link URL (found)</dt><dd className="mono br">{selected.detected_target_url ?? "—"}</dd></div>
              <div><dt>Anchor (found)</dt><dd>{selected.detected_anchor_text ?? "—"}</dd></div>
              <div><dt>Anchor (expected)</dt><dd>{selected.expected_anchor_text ?? "—"}</dd></div>
              <div><dt>Rel (found)</dt><dd className="mono">{selected.detected_rel?.join(" ") || "—"}</dd></div>
              <div><dt>dofollow / nofollow</dt><dd>{yn(selected.is_dofollow)} / {yn(selected.is_nofollow)}</dd></div>
              <div><dt>sponsored / ugc</dt><dd>{yn(selected.is_sponsored)} / {yn(selected.is_ugc)}</dd></div>
              <div><dt>Link type</dt><dd>{selected.link_type ?? "—"}</dd></div>
              <div><dt>Image alt</dt><dd>{selected.link_type === "image" ? (selected.image_alt || "(empty)") : "—"}</dd></div>
              <div><dt>HTTP source / target</dt><dd className="mono">{httpPair(selected)}</dd></div>
              <div><dt>First seen</dt><dd>{fmtLong(selected.first_checked_at)}</dd></div>
              <div><dt>Last seen</dt><dd>{fmtLong(selected.last_seen_at)}</dd></div>
              <div><dt>Last checked</dt><dd>{fmtLong(selected.last_checked_at)}</dd></div>
              <div><dt>Lost at</dt><dd>{fmtLong(selected.lost_at)}</dd></div>
              <div><dt>Cost</dt><dd>{selected.cost ? `${selected.cost} ${selected.currency ?? ""}` : "—"}</dd></div>
            </dl>
            {occurrences.length > 0 && (
              <>
                <h3 className="drawer-sub">Occurrences ({occurrences.length})</h3>
                <ul className="occ-list">
                  {occurrences.map((o) => (
                    <li key={o.id} className="occ">
                      <div className="occ-top">
                        <span className="occ-anchor">
                          {o.anchor_text ||
                            (o.link_type === "image"
                              ? `[image${o.image_alt ? ": " + o.image_alt : ""}]`
                              : "—")}
                        </span>
                        <span className="occ-place">{o.placement}</span>
                      </div>
                      <div className="rel-chips">
                        {o.link_type === "image" && <span className="rel-chip">image</span>}
                        {o.is_nofollow ? (
                          <span className="rel-chip rel-nofollow">nofollow</span>
                        ) : (
                          <span className="rel-chip rel-dofollow">dofollow</span>
                        )}
                        {o.is_sponsored && <span className="rel-chip rel-sponsored">sponsored</span>}
                        {o.is_ugc && <span className="rel-chip rel-ugc">ugc</span>}
                      </div>
                      {o.surrounding_text && <div className="occ-ctx muted">“{o.surrounding_text}”</div>}
                    </li>
                  ))}
                </ul>
              </>
            )}
            <h3 className="drawer-sub">Tier 2 — reinforcing links ({tier2.length})</h3>
            {tier2.length === 0 ? (
              <p className="muted">No Tier-2 links yet.</p>
            ) : (
              <ul className="occ-list">
                {tier2.map((t) => (
                  <li key={t.id} className="occ">
                    <div className="occ-top">
                      <span className="occ-anchor mono">{t.source_domain}</span>
                      <span className="badge" style={{ "--st": statusColor(t.status) } as CSSProperties}><span className="dot" />{statusLabel(t.status)}</span>
                    </div>
                  </li>
                ))}
              </ul>
            )}
            <div className="tags-add">
              <textarea
                style={{ flex: 1, minHeight: 44, fontFamily: "ui-monospace, monospace", fontSize: 12, background: "var(--bg)", color: "var(--text)", border: "1px solid var(--border)", borderRadius: 6, padding: "6px 8px" }}
                placeholder="source_url,anchor,rel (one per line)"
                value={t2csv}
                onChange={(e) => setT2csv(e.target.value)}
              />
              <button className="btn sm" onClick={() => void importTier2()}>Add T2</button>
            </div>
            {t2msg && <p className="muted" style={{ marginTop: -6, marginBottom: 10 }}>{t2msg}</p>}
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

function fmtShort(s?: string | null): string {
  if (!s) return "—";
  const d = new Date(s);
  return Number.isNaN(d.getTime()) ? "—" : d.toLocaleDateString(undefined, { month: "short", day: "numeric" });
}
function fmtLong(s?: string | null): string {
  if (!s) return "—";
  const d = new Date(s);
  return Number.isNaN(d.getTime()) ? "—" : d.toLocaleString();
}
function httpPair(b: BacklinkView): string {
  const s = b.source_http_status ?? null;
  const t = b.target_http_status ?? null;
  if (s == null && t == null) return "—";
  return `${s ?? "—"} / ${t ?? "—"}`;
}
function yn(v?: boolean | null): string {
  return v == null ? "—" : v ? "yes" : "no";
}
function RelChips({ b }: { b: BacklinkView }) {
  const rel = b.detected_rel;
  if (!rel || rel.length === 0) {
    return <span className="rel-chip rel-expected" title="Expected (not yet observed)">{b.expected_rel_type}</span>;
  }
  return (
    <span className="rel-chips">
      {rel.map((r) => (
        <span key={r} className={`rel-chip rel-${r}`}>{r}</span>
      ))}
    </span>
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
