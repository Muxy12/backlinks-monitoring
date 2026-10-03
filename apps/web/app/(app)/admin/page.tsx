"use client";

import { useCallback, useEffect, useState } from "react";
import { useRouter } from "next/navigation";
import {
  api,
  adminApi,
  ADMIN_PLANS,
  type MeResponse,
  type AdminStats,
  type AdminOrg,
  type AdminUser,
  type AdminAudit,
} from "@/lib/api";
import "@/app/auth.css";
import "../dashboard/dashboard.css";
import "./admin.css";

const PLAN_LABEL: Record<string, string> = {
  starter: "Starter",
  pro: "Pro",
  agency: "Agency",
  agency_plus: "Agency+",
};

// Relative time ("3 min ago") with the exact timestamp available on hover.
function relTime(iso: string): { rel: string; exact: string } {
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return { rel: "—", exact: "" };
  const exact = d.toLocaleString(undefined, { dateStyle: "medium", timeStyle: "medium" });
  const s = Math.round((Date.now() - d.getTime()) / 1000);
  const rel =
    s < 45 ? "just now" :
    s < 3600 ? `${Math.round(s / 60)} min ago` :
    s < 86400 ? `${Math.round(s / 3600)} h ago` :
    s < 2592000 ? `${Math.round(s / 86400)} d ago` :
    d.toLocaleDateString(undefined, { month: "short", day: "numeric", year: "numeric" });
  return { rel, exact };
}
function Time({ iso }: { iso: string }) {
  const { rel, exact } = relTime(iso);
  return <span className="admin-time" title={exact}>{rel}</span>;
}

// Copyable internal id — available for support/debugging without dominating the UI.
function IdChip({ id }: { id: string }) {
  const [copied, setCopied] = useState(false);
  const short = id.length > 10 ? `${id.slice(0, 8)}…` : id;
  return (
    <button
      type="button"
      className={`idchip${copied ? " copied" : ""}`}
      title={`${id} — click to copy`}
      onClick={async (e) => {
        e.stopPropagation();
        try {
          await navigator.clipboard.writeText(id);
          setCopied(true);
          setTimeout(() => setCopied(false), 1200);
        } catch { /* clipboard unavailable */ }
      }}
    >
      {copied ? "copied" : short}
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" aria-hidden="true">
        <rect x="9" y="9" width="11" height="11" rx="2" /><path d="M5 15V5a2 2 0 0 1 2-2h10" />
      </svg>
    </button>
  );
}

export default function AdminPage() {
  const router = useRouter();
  const [me, setMe] = useState<MeResponse | null>(null);
  const [loading, setLoading] = useState(true);
  const [denied, setDenied] = useState(false);
  const [stats, setStats] = useState<AdminStats | null>(null);
  const [orgs, setOrgs] = useState<AdminOrg[]>([]);
  const [users, setUsers] = useState<AdminUser[]>([]);
  const [audit, setAudit] = useState<AdminAudit[]>([]);
  const [tab, setTab] = useState<"orgs" | "users" | "audit">("orgs");
  const [busyId, setBusyId] = useState<string | null>(null);
  const [err, setErr] = useState<string | null>(null);
  const [q, setQ] = useState("");

  const loadAll = useCallback(async () => {
    const [s, o, u, a] = await Promise.all([
      adminApi.stats(),
      adminApi.organizations(),
      adminApi.users(),
      adminApi.audit(),
    ]);
    if (s.ok) setStats(s.data);
    if (o.ok) setOrgs(o.data);
    if (u.ok) setUsers(u.data);
    if (a.ok) setAudit(a.data);
    if (!s.ok && (s.status === 404 || s.status === 401)) setDenied(true);
  }, []);

  useEffect(() => {
    api.me().then(async (res) => {
      if (!res.ok) {
        router.push("/login");
        return;
      }
      setMe(res.data);
      if (!res.data.user.is_admin) {
        setDenied(true);
        setLoading(false);
        return;
      }
      await loadAll();
      setLoading(false);
    });
  }, [router, loadAll]);

  const refreshOrgs = async () => {
    const [o, s, a] = await Promise.all([
      adminApi.organizations(),
      adminApi.stats(),
      adminApi.audit(),
    ]);
    if (o.ok) setOrgs(o.data);
    if (s.ok) setStats(s.data);
    if (a.ok) setAudit(a.data);
  };

  const toggleAdmin = async (u: AdminUser) => {
    const next = !u.is_platform_admin;
    if (
      me &&
      u.id === me.user.id &&
      !next &&
      !confirm("Revoke your OWN platform-admin access? You may lose access to this page.")
    ) {
      return;
    }
    setBusyId(u.id);
    setErr(null);
    const res = await adminApi.setAdmin(u.id, next);
    setBusyId(null);
    if (!res.ok) {
      setErr(res.detail);
      return;
    }
    setUsers((list) => list.map((x) => (x.id === u.id ? { ...x, is_platform_admin: next } : x)));
  };

  const toggleSuspend = async (o: AdminOrg) => {
    const next = !o.suspended;
    if (
      next &&
      !confirm(`Suspend "${o.name}"? Its members will be locked out until you unsuspend it.`)
    ) {
      return;
    }
    setBusyId(o.id);
    setErr(null);
    const res = await adminApi.setOrgSuspended(o.id, next);
    setBusyId(null);
    if (!res.ok) {
      setErr(res.detail);
      return;
    }
    await refreshOrgs();
  };

  const changePlan = async (o: AdminOrg, plan: string) => {
    if (plan === o.plan) return;
    setBusyId(o.id);
    setErr(null);
    const res = await adminApi.setOrgPlan(o.id, plan);
    setBusyId(null);
    if (!res.ok) {
      setErr(res.detail);
      return;
    }
    await refreshOrgs();
  };

  const removeOrg = async (o: AdminOrg) => {
    const typed = prompt(
      `PERMANENTLY delete "${o.name}" and ALL its data (projects, backlinks, members)?\n` +
        `This cannot be undone. Type the workspace slug to confirm:\n\n${o.slug}`,
    );
    if (typed === null) return;
    if (typed.trim() !== o.slug) {
      setErr("Slug did not match — deletion cancelled.");
      return;
    }
    setBusyId(o.id);
    setErr(null);
    const res = await adminApi.deleteOrg(o.id);
    setBusyId(null);
    if (!res.ok) {
      setErr(res.detail);
      return;
    }
    await refreshOrgs();
  };

  if (loading) {
    return (
      <div className="dash">
        <div className="page-head">
          <h1>Admin</h1>
        </div>
        <p className="muted">Loading…</p>
      </div>
    );
  }

  if (denied) {
    return (
      <div className="dash">
        <div className="page-head">
          <h1>Admin</h1>
        </div>
        <div className="panel">
          <p className="muted" style={{ margin: 0 }}>
            This area is for platform operators only.
          </p>
          <p style={{ marginTop: 12 }}>
            <a className="btn" href="/dashboard">
              Back to dashboard
            </a>
          </p>
        </div>
      </div>
    );
  }

  const term = q.trim().toLowerCase();
  const match = (...vals: (string | null | undefined)[]) =>
    vals.some((v) => !!v && v.toLowerCase().includes(term));
  const fOrgs = term ? orgs.filter((o) => match(o.name, o.slug, o.owner_email)) : orgs;
  const fUsers = term ? users.filter((u) => match(u.name, u.email)) : users;
  const fAudit = term ? audit.filter((a) => match(a.actor_email, a.action, a.org_name, a.resource_type)) : audit;
  const livePct = stats && stats.backlinks ? Math.round((stats.backlinks_live / stats.backlinks) * 100) : null;
  const lostPct = stats && stats.backlinks ? Math.round((stats.backlinks_lost / stats.backlinks) * 100) : null;
  const searchPlaceholder =
    tab === "orgs" ? "Search workspaces, slugs, owners…" :
    tab === "users" ? "Search users by name or email…" :
    "Search audit by actor, action, workspace…";

  return (
    <div className="dash">
      <div className="page-head">
        <h1>Admin <span className="admin-tag">Operator</span></h1>
        <span className="muted">Platform backoffice — every workspace and account on this instance.</span>
      </div>

      {stats && (
        <div className="admin-stats">
          <div className="admin-stat"><div className="k">Workspaces</div><div className="v">{stats.organizations.toLocaleString()}</div></div>
          <div className="admin-stat"><div className="k">Users</div><div className="v">{stats.users.toLocaleString()}</div></div>
          <div className="admin-stat"><div className="k">Projects</div><div className="v">{stats.projects.toLocaleString()}</div></div>
          <div className="admin-stat"><div className="k">Backlinks</div><div className="v">{stats.backlinks.toLocaleString()}</div></div>
          <div className="admin-stat"><div className="k">Live</div><div className="v">{stats.backlinks_live.toLocaleString()}</div>{livePct != null && <div className="sub ok">{livePct}% of tracked</div>}</div>
          <div className="admin-stat"><div className="k">Lost</div><div className="v">{stats.backlinks_lost.toLocaleString()}</div>{lostPct != null && <div className="sub bad">{lostPct}% of tracked</div>}</div>
        </div>
      )}

      {err && <p className="error">{err}</p>}

      <div className="panel">
        <div className="admin-toolbar">
          <div className="admin-tabs" role="tablist">
            <button role="tab" aria-selected={tab === "orgs"} className={tab === "orgs" ? "on" : ""} onClick={() => setTab("orgs")}>Workspaces <span className="n">{orgs.length}</span></button>
            <button role="tab" aria-selected={tab === "users"} className={tab === "users" ? "on" : ""} onClick={() => setTab("users")}>Users <span className="n">{users.length}</span></button>
            <button role="tab" aria-selected={tab === "audit"} className={tab === "audit" ? "on" : ""} onClick={() => setTab("audit")}>Audit <span className="n">{audit.length}</span></button>
          </div>
          <input className="admin-search" placeholder={searchPlaceholder} value={q} onChange={(e) => setQ(e.target.value)} aria-label="Search" />
        </div>

        {tab === "orgs" && (
          <div className="bl-scroll">
            <table className="bl-table admin-table">
              <thead>
                <tr>
                  <th>Workspace</th>
                  <th>Owner</th>
                  <th>Plan</th>
                  <th>Members</th>
                  <th>Projects</th>
                  <th>Backlinks</th>
                  <th>Status</th>
                  <th style={{ textAlign: "right" }}></th>
                </tr>
              </thead>
              <tbody>
                {fOrgs.map((o) => (
                  <tr key={o.id} style={o.suspended ? { opacity: 0.55 } : undefined}>
                    <td>
                      <div className="admin-name">{o.name}</div>
                      <div className="admin-sub">/{o.slug}</div>
                    </td>
                    <td className="admin-email">{o.owner_email ?? "—"}</td>
                    <td>
                      <select
                        value={o.plan}
                        disabled={busyId === o.id}
                        onChange={(e) => changePlan(o, e.target.value)}
                        aria-label={`Plan for ${o.name}`}
                      >
                        {ADMIN_PLANS.map((p) => (
                          <option key={p} value={p}>{PLAN_LABEL[p] ?? p}</option>
                        ))}
                      </select>
                    </td>
                    <td className="mono">{o.members}</td>
                    <td className="mono">{o.projects}</td>
                    <td className="mono">{o.backlinks.toLocaleString()}</td>
                    <td>
                      {o.suspended
                        ? <span className="abadge bad">suspended</span>
                        : <span className="abadge ok">active</span>}
                    </td>
                    <td style={{ whiteSpace: "nowrap", textAlign: "right" }}>
                      <IdChip id={o.id} />{" "}
                      <button className="btn sm" disabled={busyId === o.id} onClick={() => toggleSuspend(o)}>
                        {o.suspended ? "Unsuspend" : "Suspend"}
                      </button>{" "}
                      <button className="btn sm danger" disabled={busyId === o.id} onClick={() => removeOrg(o)}>
                        Delete
                      </button>
                    </td>
                  </tr>
                ))}
                {fOrgs.length === 0 && (
                  <tr>
                    <td colSpan={8} className="muted">
                      {term ? "No workspaces match your search." : "No organizations yet."}
                    </td>
                  </tr>
                )}
              </tbody>
            </table>
          </div>
        )}

        {tab === "users" && (
          <div className="bl-scroll">
            <table className="bl-table admin-table">
              <thead>
                <tr>
                  <th>User</th>
                  <th>Email</th>
                  <th>Orgs</th>
                  <th>Verified</th>
                  <th>Admin</th>
                  <th>Created</th>
                  <th style={{ textAlign: "right" }}></th>
                </tr>
              </thead>
              <tbody>
                {fUsers.map((u) => (
                  <tr key={u.id}>
                    <td>
                      <div className="admin-name">
                        {u.name}
                        {me && u.id === me.user.id && <span className="role-pill" style={{ marginLeft: 6 }}>you</span>}
                      </div>
                    </td>
                    <td className="admin-email">{u.email}</td>
                    <td className="mono">{u.organizations}</td>
                    <td>{u.email_verified ? <span className="abadge ok">verified</span> : <span className="muted">—</span>}</td>
                    <td>{u.is_platform_admin ? <span className="abadge info">admin</span> : <span className="muted">—</span>}</td>
                    <td><Time iso={u.created_at} /></td>
                    <td style={{ whiteSpace: "nowrap", textAlign: "right" }}>
                      <IdChip id={u.id} />{" "}
                      <button className="btn sm" disabled={busyId === u.id} onClick={() => toggleAdmin(u)}>
                        {busyId === u.id ? "…" : u.is_platform_admin ? "Revoke admin" : "Make admin"}
                      </button>
                    </td>
                  </tr>
                ))}
                {fUsers.length === 0 && (
                  <tr>
                    <td colSpan={7} className="muted">
                      {term ? "No users match your search." : "No users yet."}
                    </td>
                  </tr>
                )}
              </tbody>
            </table>
          </div>
        )}

        {tab === "audit" && (
          <div className="bl-scroll">
            <table className="bl-table admin-table">
              <thead>
                <tr>
                  <th>When</th>
                  <th>Actor</th>
                  <th>Action</th>
                  <th>Target</th>
                  <th>Detail</th>
                </tr>
              </thead>
              <tbody>
                {fAudit.map((a) => (
                  <tr key={a.id}>
                    <td><Time iso={a.created_at} /></td>
                    <td className="admin-email">{a.actor_email ?? "—"}</td>
                    <td><span className="atag">{a.action}</span></td>
                    <td className="sm">{a.org_name ?? a.resource_type ?? "—"}</td>
                    <td className="mono muted" style={{ maxWidth: 280, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }} title={a.after ?? undefined}>{a.after ?? "—"}</td>
                  </tr>
                ))}
                {fAudit.length === 0 && (
                  <tr>
                    <td colSpan={5} className="muted">
                      {term ? "No audit entries match your search." : "No audit entries yet. Admin actions (suspend, plan change, delete) are logged here."}
                    </td>
                  </tr>
                )}
              </tbody>
            </table>
          </div>
        )}
      </div>
    </div>
  );
}
