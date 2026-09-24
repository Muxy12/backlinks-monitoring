"use client";

import { useCallback, useEffect, useState } from "react";
import { useRouter } from "next/navigation";
import { api, orgApi, backlinksExportUrl, type BacklinkView, type DashboardResponse, type EventView, type MeResponse, type NotificationsResponse, type ProjectView, type TagView } from "../../lib/api";
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
  const [orgTags, setOrgTags] = useState<TagView[]>([]);
  const [blTags, setBlTags] = useState<TagView[]>([]);
  const [newTag, setNewTag] = useState("");
  const [statusFilter, setStatusFilter] = useState("");
  const [searchQ, setSearchQ] = useState("");
  const [loading, setLoading] = useState(true);

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
      const tags = await orgApi.tags(org);
      if (tags.ok) setOrgTags(tags.data);
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
    if (!orgId || !projectId) {
      setBacklinks([]);
      return;
    }
    const t = setTimeout(() => {
      orgApi
        .searchBacklinks(orgId, {
          projectId,
          status: statusFilter || undefined,
          q: searchQ.trim() || undefined,
        })
        .then((res) => {
          if (res.ok) setBacklinks(res.data);
        });
    }, 200); // debounce the search box
    return () => clearTimeout(t);
  }, [orgId, projectId, statusFilter, searchQ]);

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

  return (
    <main className="dash">
      <header className="dash-head">
        <div>
          <h1>Dashboard</h1>
          <p className="muted">{me.organizations[0]?.name ?? "No workspace"} · {me.user.email}</p>
        </div>
        <div className="head-actions">
          <a className="btn" href="/operations">Operations</a>
          <a className="btn" href="/import">Import</a>
          <a className="btn" href="/team">Team</a>
          <button className="btn bell" onClick={() => setShowNotifs((v) => !v)} aria-label="Notifications">
            🔔
            {notifs && notifs.unread > 0 && <span className="bell-badge">{notifs.unread}</span>}
          </button>
          <button className="btn" onClick={async () => { await api.logout(); router.push("/login"); }}>Log out</button>
        </div>
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
              <div className="panel-actions">
                {projects.length > 0 && (
                  <select value={projectId ?? ""} onChange={(e) => setProjectId(e.target.value)}>
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
                {orgId && (
                  <a className="btn sm" href={backlinksExportUrl(orgId, projectId ?? undefined)}>
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
              <p className="muted">No backlinks in this project yet.</p>
            ) : (
              <table className="bl-table">
                <thead>
                  <tr><th>Status</th><th>Source</th><th>Target</th><th>Anchor</th><th>Rel</th><th>Cost</th></tr>
                </thead>
                <tbody>
                  {backlinks.map((b) => (
                    <tr key={b.id} className="row-click" onClick={() => void openDetail(b)}>
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
      {showNotifs && (
        <div className="drawer-backdrop" onClick={() => setShowNotifs(false)}>
          <aside className="drawer" onClick={(e) => e.stopPropagation()}>
            <div className="panel-head">
              <h2>Notifications</h2>
              <button className="btn" onClick={() => setShowNotifs(false)}>Close</button>
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
              <span className="badge" style={{ color: statusColor(selected.status) }}>● {statusLabel(selected.status)}</span>
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

function Tile({ label, value, color }: { label: string; value: number | string; color?: string }) {
  return (
    <div className="tile">
      <div className="tile-label">{label}</div>
      <div className="tile-value" style={color ? { color } : undefined}>{value}</div>
    </div>
  );
}
