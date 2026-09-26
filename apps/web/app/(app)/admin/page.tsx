"use client";

import { useCallback, useEffect, useState } from "react";
import { useRouter } from "next/navigation";
import {
  api,
  adminApi,
  type MeResponse,
  type AdminStats,
  type AdminOrg,
  type AdminUser,
} from "@/lib/api";
import "@/app/auth.css";
import "../dashboard/dashboard.css";

export default function AdminPage() {
  const router = useRouter();
  const [me, setMe] = useState<MeResponse | null>(null);
  const [loading, setLoading] = useState(true);
  const [denied, setDenied] = useState(false);
  const [stats, setStats] = useState<AdminStats | null>(null);
  const [orgs, setOrgs] = useState<AdminOrg[]>([]);
  const [users, setUsers] = useState<AdminUser[]>([]);
  const [tab, setTab] = useState<"orgs" | "users">("orgs");
  const [busyId, setBusyId] = useState<string | null>(null);
  const [err, setErr] = useState<string | null>(null);

  const loadAll = useCallback(async () => {
    const [s, o, u] = await Promise.all([
      adminApi.stats(),
      adminApi.organizations(),
      adminApi.users(),
    ]);
    if (s.ok) setStats(s.data);
    if (o.ok) setOrgs(o.data);
    if (u.ok) setUsers(u.data);
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
    const s = await adminApi.stats();
    if (s.ok) setStats(s.data);
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
            <button
              className={`btn${tab === "orgs" ? " primary" : ""}`}
              onClick={() => setTab("orgs")}
            >
              Organizations ({orgs.length})
            </button>
            <button
              className={`btn${tab === "users" ? " primary" : ""}`}
              onClick={() => setTab("users")}
            >
              Users ({users.length})
            </button>
          </div>
        </div>

        {tab === "orgs" ? (
          <div style={{ overflowX: "auto" }}>
            <table className="bl-table">
              <thead>
                <tr>
                  <th>Workspace</th>
                  <th>Owner</th>
                  <th>Members</th>
                  <th>Projects</th>
                  <th>Backlinks</th>
                  <th>Created</th>
                </tr>
              </thead>
              <tbody>
                {orgs.map((o) => (
                  <tr key={o.id}>
                    <td>
                      <strong>{o.name}</strong>
                      <div className="muted mono sm">/{o.slug}</div>
                    </td>
                    <td className="mono sm">{o.owner_email ?? "—"}</td>
                    <td>{o.members}</td>
                    <td>{o.projects}</td>
                    <td>{o.backlinks}</td>
                    <td className="muted sm">{new Date(o.created_at).toLocaleDateString()}</td>
                  </tr>
                ))}
                {orgs.length === 0 && (
                  <tr>
                    <td colSpan={6} className="muted">
                      No organizations yet.
                    </td>
                  </tr>
                )}
              </tbody>
            </table>
          </div>
        ) : (
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
                      <button
                        className="btn sm"
                        disabled={busyId === u.id}
                        onClick={() => toggleAdmin(u)}
                      >
                        {busyId === u.id
                          ? "…"
                          : u.is_platform_admin
                            ? "Revoke admin"
                            : "Make admin"}
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
      </div>
    </div>
  );
}
