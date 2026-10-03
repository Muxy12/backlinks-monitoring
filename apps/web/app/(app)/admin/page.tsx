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

  // Derived operational metrics — computed from the lists the admin already loads
  // (real data, no fabrication). At scale these should become backend aggregates;
  // the coverage note is shown when the loaded lists are not complete.
  const WEEK = 7 * 86400000;
  const inLastWeek = (iso: string) => Date.now() - new Date(iso).getTime() < WEEK;
  const newUsers7 = users.filter((u) => inLastWeek(u.created_at)).length;
  const newOrgs7 = orgs.filter((o) => inLastWeek(o.created_at)).length;
  const activatedOrgs = orgs.filter((o) => o.backlinks > 0).length;
  const activatedPct = orgs.length ? Math.round((activatedOrgs / orgs.length) * 100) : 0;
  const suspendedCount = orgs.filter((o) => o.suspended).length;
  const planMix = ADMIN_PLANS.map((p) => ({ plan: p, n: orgs.filter((o) => o.plan === p).length }));
  const orgsComplete = !stats || orgs.length >= stats.organizations;

  return (
    <div className="dash">
      <div className="page-head">
        <h1>Admin <span className="admin-tag">Operator</span></h1>
        <span className="muted">Platform backoffice — every workspace and account on this instance.</span>
      </div>

      {stats && (
        <div className="admin-stats">
          <div className="admin-stat"><div className="k">Workspaces</div><div className="v">{stats.organizations.toLocaleString()}</div><div className={`delta${newOrgs7 ? " up" : ""}`}>{newOrgs7 ? `+${newOrgs7} new · 7d` : "none new · 7d"}</div></div>
          <div className="admin-stat"><div className="k">Users</div><div className="v">{stats.users.toLocaleString()}</div><div className={`delta${newUsers7 ? " up" : ""}`}>{newUsers7 ? `+${newUsers7} new · 7d` : "none new · 7d"}</div></div>
          <div className="admin-stat"><div className="k">Activated</div><div className="v">{activatedPct}%</div><div className="sub">{activatedOrgs} of {orgs.length} have backlinks</div></div>
          <div className="admin-stat"><div className="k">Suspended</div><div className="v">{suspendedCount.toLocaleString()}</div><div className={`sub${suspendedCount ? " bad" : ""}`}>{suspendedCount ? "needs review" : "none"}</div></div>
          <div className="admin-stat"><div className="k">Backlinks</div><div className="v">{stats.backlinks.toLocaleString()}</div><div className="sub">monitored platform-wide</div></div>
          <div className="admin-stat"><div className="k">Live</div><div className="v">{livePct != null ? `${livePct}%` : "—"}</div>{lostPct != null && <div className="sub bad">{stats.backlinks_lost.toLocaleString()} lost · {lostPct}%</div>}</div>
        </div>
      )}

      {stats && orgs.length > 0 && (
        <>
          <div className="ops-section-h">
            <h2>Plan distribution</h2>
            <span className="note">Plan tier from active subscription (paying or admin-comped){orgsComplete ? "" : ` · ${orgs.length} of ${stats.organizations} loaded`}</span>
          </div>
          <div className="plan-mix">
            {planMix.map((pm) => (
              <span key={pm.plan} className={`plan-chip${pm.n === 0 ? " muted" : ""}`}>
                <span className="pn">{PLAN_LABEL[pm.plan] ?? pm.plan}</span><span className="pc">{pm.n}</span>
              </span>
            ))}
            {suspendedCount > 0 && (
              <span className="plan-chip danger"><span className="pn">Suspended</span><span className="pc">{suspendedCount}</span></span>
            )}
          </div>
        </>
      )}

      <div className="ops-section-h">
        <h2>Operational health</h2>
        <span className="note">What this console can&rsquo;t yet answer — and what each needs.</span>
      </div>
      <div className="ops-gap">
        {[
          { t: "Monitoring — checks running & failing", ready: true, d: "Every check is recorded (backlink_checks: found, error_kind, duration_ms). Add an aggregate endpoint for checks today / failed / avg duration." },
          { t: "Queue health — pending · failed · dead · stuck", ready: true, d: "The jobs table and its purpose-built stuck/dead indexes already hold this. Add a read query + endpoint." },
          { t: "Revenue — MRR · paying · canceled", ready: true, d: "subscriptions.status + the plan price catalog give paying/canceled and MRR (exclude admin-comped subs). Add a cross-tenant billing endpoint." },
          { t: "Plan-limit usage — who's near their cap", ready: true, d: "Plan limits (backlink/project/seat) are defined and live counts exist. Add an endpoint joining usage vs limit." },
          { t: "Email — alert sends & failures", ready: true, d: "alert_deliveries logs every alert email send/fail. Add an endpoint. (True delivered/bounced needs a Resend webhook.)" },
          { t: "Scheduler / last tick", ready: true, d: "/internal/tick returns per-tick counts. Surface last-tick time + processed/failed as a health endpoint." },
          { t: "Churn & renewal timing", ready: false, d: "subscriptions has period_end / canceled_at / trial_end columns, but the Stripe webhook never writes them. Needs webhook instrumentation." },
          { t: "Provider & API usage / cost", ready: false, d: "No external data provider is integrated; usage_records exists but is never populated (RecomputeUsage job has no handler). Needs metering." },
        ].map((o) => (
          <div className="ops-row" key={o.t}>
            <div className="ops-b">
              <div className="ops-t">{o.t}<span className={`ops-pill${o.ready ? " partial" : ""}`}>{o.ready ? "data ready" : "needs instrumentation"}</span></div>
              <div className="ops-d">{o.d}</div>
            </div>
          </div>
        ))}
      </div>
      <p className="ops-gap-foot">&ldquo;Data ready&rdquo; = already in the database, only needs a read endpoint. &ldquo;Needs instrumentation&rdquo; = not captured yet. Full build spec: docs/19 — admin operations instrumentation.</p>

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
