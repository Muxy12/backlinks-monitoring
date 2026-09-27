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

const PLAN_LABEL: Record<string, string> = {
  starter: "Starter",
  pro: "Pro",
  agency: "Agency",
  agency_plus: "Agency+",
};

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

  const tiles: { label: string; value: number }[] = stats
    ? [
        { label: "Organizations", value: stats.organizations },
        { label: "Users", value: stats.users },
        { label: "Projects", value: stats.projects },
        { label: "Backlinks", value: stats.backlinks },
        { label: "Live", value: stats.backlinks_live },
        { label: "Lost", value: stats.backlinks_lost },
      ]
    : [];

  return (
    <div className="dash">
      <div className="page-head">
        <h1>Admin</h1>
        <span className="muted">Platform backoffice — every workspace and account on this instance.</span>
      </div>

      <div className="tiles">
        {tiles.map((t) => (
          <div className="tile" key={t.label}>
            <div className="tile-label">{t.label}</div>
            <div className="tile-value">{t.value.toLocaleString()}</div>
          </div>
        ))}
      </div>

      {err && <p className="error">{err}</p>}

      <div className="panel">
        <div className="panel-head">
          <div className="head-actions" style={{ gap: 8 }}>
            <button className={`btn${tab === "orgs" ? " primary" : ""}`} onClick={() => setTab("orgs")}>
              Organizations ({orgs.length})
            </button>
            <button className={`btn${tab === "users" ? " primary" : ""}`} onClick={() => setTab("users")}>
              Users ({users.length})
            </button>
            <button className={`btn${tab === "audit" ? " primary" : ""}`} onClick={() => setTab("audit")}>
              Audit ({audit.length})
            </button>
          </div>
        </div>

        {tab === "orgs" && (
          <div style={{ overflowX: "auto" }}>
            <table className="bl-table">
              <thead>
                <tr>
                  <th>Workspace</th>
                  <th>Owner</th>
                  <th>Plan</th>
                  <th>Members</th>
                  <th>Projects</th>
                  <th>Backlinks</th>
                  <th>Status</th>
                  <th></th>
                </tr>
              </thead>
              <tbody>
                {orgs.map((o) => (
                  <tr key={o.id} style={o.suspended ? { opacity: 0.6 } : undefined}>
                    <td>
                      <strong>{o.name}</strong>
                      <div className="muted mono sm">/{o.slug}</div>
                    </td>
                    <td className="mono sm">{o.owner_email ?? "—"}</td>
                    <td>
                      <select
                        value={o.plan}
                        disabled={busyId === o.id}
                        onChange={(e) => changePlan(o, e.target.value)}
                        aria-label={`Plan for ${o.name}`}
                      >
                        {ADMIN_PLANS.map((p) => (
                          <option key={p} value={p}>
                            {PLAN_LABEL[p] ?? p}
                          </option>
                        ))}
                      </select>
                    </td>
                    <td>{o.members}</td>
                    <td>{o.projects}</td>
                    <td>{o.backlinks}</td>
                    <td>
                      {o.suspended ? (
                        <span className="badge" style={{ background: "#fee2e2", color: "#991b1b" }}>
                          suspended
                        </span>
                      ) : (
                        <span className="badge" style={{ background: "#dcfce7", color: "#166534" }}>
                          active
                        </span>
                      )}
                    </td>
                    <td style={{ whiteSpace: "nowrap" }}>
                      <button
                        className="btn sm"
                        disabled={busyId === o.id}
                        onClick={() => toggleSuspend(o)}
                      >
                        {o.suspended ? "Unsuspend" : "Suspend"}
                      </button>{" "}
                      <button
                        className="btn sm"
                        style={{ color: "#b91c1c", borderColor: "#fecaca" }}
                        disabled={busyId === o.id}
                        onClick={() => removeOrg(o)}
                      >
                        Delete
                      </button>
                    </td>
                  </tr>
                ))}
                {orgs.length === 0 && (
                  <tr>
                    <td colSpan={8} className="muted">
                      No organizations yet.
                    </td>
                  </tr>
                )}
              </tbody>
            </table>
          </div>
        )}

        {tab === "users" && (
          <div style={{ overflowX: "auto" }}>
            <table className="bl-table">
              <thead>
                <tr>
                  <th>User</th>
                  <th>Email</th>
                  <th>Orgs</th>
                  <th>Verified</th>
                  <th>Admin</th>
                  <th>Created</th>
                  <th></th>
                </tr>
              </thead>
              <tbody>
                {users.map((u) => (
                  <tr key={u.id}>
                    <td>
                      <strong>{u.name}</strong>
                      {me && u.id === me.user.id && (
                        <span className="role-pill" style={{ marginLeft: 6 }}>
                          you
                        </span>
                      )}
                    </td>
                    <td className="mono sm">{u.email}</td>
                    <td>{u.organizations}</td>
                    <td>{u.email_verified ? "✓" : "—"}</td>
                    <td>
                      {u.is_platform_admin ? (
                        <span className="badge" style={{ background: "#dbeafe", color: "#1e40af" }}>
                          admin
                        </span>
                      ) : (
                        <span className="muted">—</span>
                      )}
                    </td>
                    <td className="muted sm">{new Date(u.created_at).toLocaleDateString()}</td>
                    <td>
                      <button className="btn sm" disabled={busyId === u.id} onClick={() => toggleAdmin(u)}>
                        {busyId === u.id ? "…" : u.is_platform_admin ? "Revoke admin" : "Make admin"}
                      </button>
                    </td>
                  </tr>
                ))}
                {users.length === 0 && (
                  <tr>
                    <td colSpan={7} className="muted">
                      No users yet.
                    </td>
                  </tr>
                )}
              </tbody>
            </table>
          </div>
        )}

        {tab === "audit" && (
          <div style={{ overflowX: "auto" }}>
            <table className="bl-table">
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
                {audit.map((a) => (
                  <tr key={a.id}>
                    <td className="muted sm" style={{ whiteSpace: "nowrap" }}>
                      {new Date(a.created_at).toLocaleString()}
                    </td>
                    <td className="mono sm">{a.actor_email ?? "—"}</td>
                    <td>
                      <span className="badge" style={{ background: "#f1f5f9", color: "#334155" }}>
                        {a.action}
                      </span>
                    </td>
                    <td className="sm">{a.org_name ?? a.resource_type ?? "—"}</td>
                    <td className="mono sm muted">{a.after ?? "—"}</td>
                  </tr>
                ))}
                {audit.length === 0 && (
                  <tr>
                    <td colSpan={5} className="muted">
                      No audit entries yet. Admin actions (suspend, plan change, delete) are logged here.
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
