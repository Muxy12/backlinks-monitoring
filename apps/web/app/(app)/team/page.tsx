"use client";

import { useCallback, useEffect, useState } from "react";
import { useRouter } from "next/navigation";
import {
  api,
  orgApi,
  type InvitationView,
  type MemberView,
  type MeResponse,
} from "@/lib/api";
import "@/app/auth.css";
import "../dashboard/dashboard.css";
import "../operations/operations.css";

const ROLES = ["admin", "member", "viewer"];

export default function TeamPage() {
  const router = useRouter();
  const [me, setMe] = useState<MeResponse | null>(null);
  const [orgId, setOrgId] = useState<string | null>(null);
  const [members, setMembers] = useState<MemberView[]>([]);
  const [invites, setInvites] = useState<InvitationView[]>([]);
  const [email, setEmail] = useState("");
  const [role, setRole] = useState("member");
  const [error, setError] = useState<string | null>(null);
  const [lastLink, setLastLink] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [loading, setLoading] = useState(true);
  const [canManage, setCanManage] = useState(false);

  useEffect(() => {
    api.me().then((res) => {
      if (!res.ok) {
        router.push("/login");
        return;
      }
      setMe(res.data);
      const org = res.data.organizations[0];
      setOrgId(org?.id ?? null);
      setCanManage(org?.role === "owner" || org?.role === "admin");
      setLoading(false);
    });
  }, [router]);

  const reload = useCallback(async (org: string, manage: boolean) => {
    const m = await orgApi.members(org);
    if (m.ok) setMembers(m.data);
    if (manage) {
      const inv = await orgApi.invitations(org);
      if (inv.ok) setInvites(inv.data);
    }
  }, []);

  useEffect(() => {
    if (orgId) void reload(orgId, canManage);
  }, [orgId, canManage, reload]);

  async function invite(e: React.FormEvent) {
    e.preventDefault();
    setError(null);
    setLastLink(null);
    if (!orgId) return;
    setBusy(true);
    const res = await orgApi.createInvitation(orgId, email.trim(), role);
    setBusy(false);
    if (!res.ok) {
      setError(res.detail);
      return;
    }
    setEmail("");
    if (res.data.token) {
      const url = `${window.location.origin}/invite?token=${encodeURIComponent(res.data.token)}`;
      setLastLink(url);
    }
    void reload(orgId, canManage);
  }

  async function revoke(id: string) {
    if (!orgId) return;
    const res = await orgApi.revokeInvitation(orgId, id);
    if (res.ok) void reload(orgId, canManage);
  }

  if (loading) return <div className="auth-wrap"><p style={{ color: "var(--muted)" }}>Loading…</p></div>;
  if (!me) return null;

  return (
    <main className="dash">
      <header className="page-head">
        <h1>Team</h1>
        <p className="muted">{me.organizations[0]?.name ?? "No workspace"}</p>
      </header>

      {!orgId ? (
        <p className="muted">Create a workspace first.</p>
      ) : (
        <div className="ops-grid">
          <section className="panel">
            <div className="panel-head"><h2>Members</h2></div>
            <table className="bl-table">
              <thead><tr><th>Name</th><th>Email</th><th>Role</th></tr></thead>
              <tbody>
                {members.map((m) => (
                  <tr key={m.user_id}>
                    <td>{m.name}</td>
                    <td className="mono">{m.email}</td>
                    <td>{m.role}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </section>

          {canManage && (
            <section className="panel">
              <div className="panel-head"><h2>Invite</h2></div>
              <form onSubmit={invite} className="ops-form">
                {error && <div className="auth-error">{error}</div>}
                <label className="auth-field">
                  <span>Email</span>
                  <input type="email" value={email} onChange={(e) => setEmail(e.target.value)} required />
                </label>
                <label className="auth-field">
                  <span>Role</span>
                  <select value={role} onChange={(e) => setRole(e.target.value)}>
                    {ROLES.map((r) => <option key={r} value={r}>{r}</option>)}
                  </select>
                </label>
                <button className="auth-btn" type="submit" disabled={busy}>{busy ? "Inviting…" : "Send invite"}</button>
              </form>

              {lastLink && (
                <div className="invite-link">
                  <p className="muted">Invite link (share it — email delivery is coming):</p>
                  <code>{lastLink}</code>
                </div>
              )}

              {invites.length > 0 && (
                <>
                  <div className="panel-head" style={{ marginTop: 16 }}><h2>Pending</h2></div>
                  <ul className="ops-list">
                    {invites.map((i) => (
                      <li key={i.id}>
                        <div className="ops-row" style={{ cursor: "default" }}>
                          <span className="mono">{i.email} · {i.role}</span>
                          <button className="btn sm" onClick={() => void revoke(i.id)}>Revoke</button>
                        </div>
                      </li>
                    ))}
                  </ul>
                </>
              )}
            </section>
          )}
        </div>
      )}
    </main>
  );
}
