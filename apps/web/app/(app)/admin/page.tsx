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
  type AdminOps,
  type AdminBilling,
  type AdminSubscription,
  type AdminAnalytics,
  type AdminWaitlist,
  type AdminUsageLimit,
  type AdminFeedback,
} from "@/lib/api";
import "@/app/auth.css";
import "../dashboard/dashboard.css";
import "./admin.css";

const PLAN_LABEL: Record<string, string> = {
  starter: "Starter",
  pro: "Pro",
  business: "Business",
  agency: "Agency",
};

// Relative time ("3 min ago") with the exact timestamp available on hover.
function relTime(iso: string): { rel: string; exact: string } {
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return { rel: "-", exact: "" };
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

// Share of one funnel stage against an earlier one ("-" when the base is 0).
function pctStr(n: number, d: number): string {
  return d > 0 ? `${Math.round((n / d) * 100)}%` : "-";
}

// Copyable internal id, available for support/debugging without dominating the UI.
function IdChip({ id }: { id: string }) {
  const [copied, setCopied] = useState(false);
  const short = id.length > 10 ? `${id.slice(0, 8)}…` : id;
  return (
    <button
      type="button"
      className={`idchip${copied ? " copied" : ""}`}
      title={`${id}, click to copy`}
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
  const [ops, setOps] = useState<AdminOps | null>(null);
  const [billing, setBilling] = useState<AdminBilling | null>(null);
  const [subs, setSubs] = useState<AdminSubscription[]>([]);
  const [analytics, setAnalytics] = useState<AdminAnalytics | null>(null);
  const [waitlist, setWaitlist] = useState<AdminWaitlist | null>(null);
  const [feedback, setFeedback] = useState<AdminFeedback | null>(null);
  const [usage, setUsage] = useState<AdminUsageLimit[]>([]);
  const [tab, setTab] = useState<"orgs" | "subscriptions" | "users" | "audit">("orgs");
  const [busyId, setBusyId] = useState<string | null>(null);
  const [err, setErr] = useState<string | null>(null);
  const [q, setQ] = useState("");
  const [detail, setDetail] = useState<AdminOrg | null>(null);

  const loadAll = useCallback(async () => {
    const [s, o, u, a, op, bl, sb, an, ug, wl, fb] = await Promise.all([
      adminApi.stats(),
      adminApi.organizations(),
      adminApi.users(),
      adminApi.audit(),
      adminApi.ops(),
      adminApi.billing(),
      adminApi.subscriptions(),
      adminApi.analytics(),
      adminApi.usageLimits(),
      adminApi.waitlist(),
      adminApi.feedback(),
    ]);
    if (s.ok) setStats(s.data);
    if (o.ok) setOrgs(o.data);
    if (u.ok) setUsers(u.data);
    if (a.ok) setAudit(a.data);
    // Ops endpoints (docs/19) may 404 until the backend ships them; the panels
    // fall back to the "data ready" description when absent.
    if (op.ok) setOps(op.data);
    if (bl.ok) setBilling(bl.data);
    if (sb.ok) setSubs(sb.data);
    if (an.ok) setAnalytics(an.data);
    if (ug.ok) setUsage(ug.data);
    if (wl.ok) setWaitlist(wl.data);
    if (fb.ok) setFeedback(fb.data);
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
    const [o, s, a, bl, sb] = await Promise.all([
      adminApi.organizations(),
      adminApi.stats(),
      adminApi.audit(),
      adminApi.billing(),
      adminApi.subscriptions(),
    ]);
    if (o.ok) setOrgs(o.data);
    if (s.ok) setStats(s.data);
    if (a.ok) setAudit(a.data);
    if (bl.ok) setBilling(bl.data);
    if (sb.ok) setSubs(sb.data);
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
      setErr("Slug did not match, deletion cancelled.");
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

  const resendConfirmations = async () => {
    if (!waitlist || waitlist.pending === 0) return;
    if (!confirm(`Send the confirmation email to ${waitlist.pending} waitlist signup(s) who haven't received one yet?`)) {
      return;
    }
    setBusyId("waitlist-resend");
    setErr(null);
    const res = await adminApi.resendWaitlistConfirmations();
    setBusyId(null);
    if (!res.ok) {
      setErr(res.detail);
      return;
    }
    const wl = await adminApi.waitlist();
    if (wl.ok) setWaitlist(wl.data);
    alert(`Sent ${res.data.sent}${res.data.failed ? ` · ${res.data.failed} failed` : ""}. ${res.data.remaining} still pending.`);
  };

  const syncAudience = async () => {
    if (!waitlist || waitlist.total === 0) return;
    if (!confirm(`Push all ${waitlist.total} waitlist email(s) into the Resend audience? Existing contacts are skipped.`)) {
      return;
    }
    setBusyId("waitlist-sync");
    setErr(null);
    const res = await adminApi.syncWaitlistAudience();
    setBusyId(null);
    if (!res.ok) {
      setErr(res.detail);
      return;
    }
    const d = res.data;
    let text = `Synced ${d.synced} of ${d.total} to Resend${d.failed ? ` · ${d.failed} failed` : ""}.\nAudience: ${d.audience}`;
    if (d.error) text += `\n\nFirst error: ${d.error}`;
    alert(text);
  };

  const changeFeedbackStatus = async (id: string, status: string) => {
    setBusyId(`fb-${id}`);
    setErr(null);
    const res = await adminApi.setFeedbackStatus(id, status);
    setBusyId(null);
    if (!res.ok) {
      setErr(res.detail);
      return;
    }
    // Reflect the new status locally without a full reload.
    setFeedback((cur) =>
      cur
        ? {
            ...cur,
            open: cur.items.filter((i) => (i.id === id ? status : i.status) === "open").length,
            items: cur.items.map((i) => (i.id === id ? { ...i, status } : i)),
          }
        : cur,
    );
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
  const fSubs = term ? subs.filter((s) => match(s.name, s.owner_email, s.plan, s.status)) : subs;
  const fUsers = term ? users.filter((u) => match(u.name, u.email)) : users;
  const fAudit = term ? audit.filter((a) => match(a.actor_email, a.action, a.org_name, a.resource_type)) : audit;
  const livePct = stats && stats.backlinks ? Math.round((stats.backlinks_live / stats.backlinks) * 100) : null;
  const lostPct = stats && stats.backlinks ? Math.round((stats.backlinks_lost / stats.backlinks) * 100) : null;
  const searchPlaceholder =
    tab === "orgs" ? "Search workspaces, slugs, owners…" :
    tab === "subscriptions" ? "Search by workspace, owner, plan, status…" :
    tab === "users" ? "Search users by name or email…" :
    "Search audit by actor, action, workspace…";

  // Derived operational metrics, computed from the lists the admin already loads
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
  const nearLimit = usage
    .filter((u) => u.pct != null && u.pct >= 80)
    .sort((a, b) => (b.pct ?? 0) - (a.pct ?? 0));

  return (
    <div className="dash">
      <div className="page-head">
        <h1>Admin <span className="admin-tag">Operator</span></h1>
        <span className="muted">Platform backoffice, every workspace and account on this instance.</span>
      </div>

      {stats && (
        <div className="admin-stats">
          <div className="admin-stat"><div className="k">Workspaces</div><div className="v">{stats.organizations.toLocaleString()}</div><div className={`delta${newOrgs7 ? " up" : ""}`}>{newOrgs7 ? `+${newOrgs7} new · 7d` : "none new · 7d"}</div></div>
          <div className="admin-stat"><div className="k">Users</div><div className="v">{stats.users.toLocaleString()}</div><div className={`delta${newUsers7 ? " up" : ""}`}>{newUsers7 ? `+${newUsers7} new · 7d` : "none new · 7d"}</div></div>
          <div className="admin-stat"><div className="k">Activated</div><div className="v">{activatedPct}%</div><div className="sub">{activatedOrgs} of {orgs.length} have backlinks</div></div>
          <div className="admin-stat"><div className="k">Suspended</div><div className="v">{suspendedCount.toLocaleString()}</div><div className={`sub${suspendedCount ? " bad" : ""}`}>{suspendedCount ? "needs review" : "none"}</div></div>
          <div className="admin-stat"><div className="k">Backlinks</div><div className="v">{stats.backlinks.toLocaleString()}</div><div className="sub">monitored platform-wide</div></div>
          <div className="admin-stat"><div className="k">Live</div><div className="v">{livePct != null ? `${livePct}%` : "-"}</div>{lostPct != null && <div className="sub bad">{stats.backlinks_lost.toLocaleString()} lost · {lostPct}%</div>}</div>
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

      {ops ? (
        <>
          <div className="ops-section-h"><h2>System &amp; queue</h2><span className="note">jobs · live</span></div>
          <div className="admin-stats">
            <div className="admin-stat"><div className="k">Pending</div><div className="v">{ops.queue.pending.toLocaleString()}</div></div>
            <div className="admin-stat"><div className="k">Processing</div><div className="v">{ops.queue.processing.toLocaleString()}</div></div>
            <div className="admin-stat"><div className="k">Failed</div><div className="v">{ops.queue.failed.toLocaleString()}</div><div className={`sub${ops.queue.failed ? " bad" : ""}`}>{ops.queue.failed ? "retrying" : "none"}</div></div>
            <div className="admin-stat"><div className="k">Dead</div><div className="v">{ops.queue.dead.toLocaleString()}</div><div className={`sub${ops.queue.dead ? " bad" : ""}`}>{ops.queue.dead ? "needs a human" : "none"}</div></div>
            <div className="admin-stat"><div className="k">Last job</div><div className="v" style={{ fontSize: 15 }}>{ops.queue.last_completed_at ? <Time iso={ops.queue.last_completed_at} /> : "-"}</div><div className="sub">{ops.queue.created_last_hour} queued · 1h</div></div>
            <div className="admin-stat"><div className="k">Oldest pending</div><div className="v" style={{ fontSize: 15 }}>{ops.queue.oldest_pending_at ? <Time iso={ops.queue.oldest_pending_at} /> : "-"}</div></div>
          </div>
          <div className="ops-section-h"><h2>Monitoring &amp; delivery</h2><span className="note">last {Math.round(ops.window_secs / 3600)}h</span></div>
          <div className="admin-stats">
            <div className="admin-stat"><div className="k">Checks</div><div className="v">{ops.monitoring.checks.toLocaleString()}</div></div>
            <div className="admin-stat"><div className="k">Failed</div><div className="v">{ops.monitoring.failed.toLocaleString()}</div><div className={`sub${ops.monitoring.failed ? " bad" : ""}`}>{ops.monitoring.checks ? `${Math.round((ops.monitoring.failed / ops.monitoring.checks) * 100)}%` : "-"}</div></div>
            <div className="admin-stat"><div className="k">Avg duration</div><div className="v">{ops.monitoring.avg_duration_ms != null ? ops.monitoring.avg_duration_ms.toLocaleString() : "-"}</div><div className="sub">ms</div></div>
            <div className="admin-stat"><div className="k">Lost detected</div><div className="v">{ops.monitoring.lost_detected.toLocaleString()}</div></div>
            <div className="admin-stat"><div className="k">Alert emails</div><div className="v">{ops.email.sent.toLocaleString()}</div><div className="sub">sent</div></div>
            <div className="admin-stat"><div className="k">Email failures</div><div className="v">{ops.email.failed.toLocaleString()}</div><div className={`sub${ops.email.failed ? " bad" : ""}`}>{ops.email.failed ? "check destinations" : "none"}</div></div>
          </div>
        </>
      ) : (
        <>
          <div className="ops-section-h"><h2>Operational health</h2><span className="note">Live metrics load from the admin ops endpoints (docs/19).</span></div>
          <div className="ops-gap">
            {[
              { t: "Monitoring, checks running & failing", d: "backlink_checks holds found / error_kind / duration_ms. Served by /v1/admin/ops." },
              { t: "Queue health, pending · failed · dead", d: "The jobs table + stuck/dead indexes. Served by /v1/admin/ops." },
              { t: "Revenue, MRR · paying · canceled", d: "subscriptions + the plan price catalog. Served by /v1/admin/billing." },
              { t: "Plan-limit usage", d: "Plan caps vs live backlink counts. Served by /v1/admin/usage/limits." },
              { t: "Email, alert sends & failures", d: "alert_deliveries. Served by /v1/admin/ops." },
              { t: "Scheduler liveness", d: "Last completed job + jobs queued per hour. Served by /v1/admin/ops." },
            ].map((o) => (
              <div className="ops-row" key={o.t}>
                <div className="ops-b"><div className="ops-t">{o.t}<span className="ops-pill partial">data ready</span></div><div className="ops-d">{o.d}</div></div>
              </div>
            ))}
          </div>
        </>
      )}

      {billing && (
        <>
          <div className="ops-section-h"><h2>Revenue</h2><span className="note">USD · excludes admin-comped subscriptions</span></div>
          <div className="admin-stats">
            <div className="admin-stat"><div className="k">MRR</div><div className="v">${Math.round(billing.mrr_cents / 100).toLocaleString()}</div><div className="sub">/ month</div></div>
            <div className="admin-stat"><div className="k">ARR</div><div className="v">${Math.round((billing.mrr_cents * 12) / 100).toLocaleString()}</div><div className="sub">run-rate</div></div>
            <div className="admin-stat"><div className="k">Paying</div><div className="v">{billing.paying.toLocaleString()}</div><div className="sub">{billing.paying ? `$${Math.round(billing.mrr_cents / 100 / billing.paying).toLocaleString()} ARPA` : "-"}</div></div>
            <div className="admin-stat"><div className="k">Past due</div><div className="v">{billing.past_due.toLocaleString()}</div><div className={`sub${billing.past_due ? " bad" : ""}`}>{billing.past_due ? "dunning" : "none"}</div></div>
            <div className="admin-stat"><div className="k">Canceled</div><div className="v">{billing.canceled.toLocaleString()}</div></div>
            <div className="admin-stat"><div className="k">Comped</div><div className="v">{billing.comped.toLocaleString()}</div><div className="sub">admin-set</div></div>
          </div>
          <div className="ops-section-h"><h2>Trial funnel</h2><span className="note">10-day free trial → paid</span></div>
          <div className="admin-stats">
            <div className="admin-stat"><div className="k">Trials active</div><div className="v">{billing.trialing_active.toLocaleString()}</div><div className="sub">in progress</div></div>
            <div className="admin-stat"><div className="k">Trials expired</div><div className="v">{billing.trialing_expired.toLocaleString()}</div><div className={`sub${billing.trialing_expired ? " bad" : ""}`}>{billing.trialing_expired ? "lapsed, not converted" : "none"}</div></div>
            <div className="admin-stat"><div className="k">Converted</div><div className="v">{billing.paying.toLocaleString()}</div><div className="sub">now paying</div></div>
            <div className="admin-stat"><div className="k">Conversion</div><div className="v">{billing.conversion_pct != null ? `${billing.conversion_pct}%` : "-"}</div><div className="sub">of resolved trials</div></div>
          </div>
        </>
      )}

      {analytics && (analytics.signups > 0 || analytics.subscribed > 0) && (
        <>
          <div className="ops-section-h"><h2>Acquisition funnel</h2><span className="note">last {Math.round(analytics.window_secs / 86400)} days · from product events</span></div>
          <div className="admin-stats">
            <div className="admin-stat"><div className="k">Signups</div><div className="v">{analytics.signups.toLocaleString()}</div></div>
            <div className="admin-stat"><div className="k">Trials started</div><div className="v">{analytics.trials_started.toLocaleString()}</div><div className="sub">{pctStr(analytics.trials_started, analytics.signups)} of signups</div></div>
            <div className="admin-stat"><div className="k">Activated</div><div className="v">{analytics.activated.toLocaleString()}</div><div className="sub">{pctStr(analytics.activated, analytics.trials_started)} of trials</div></div>
            <div className="admin-stat"><div className="k">Subscribed</div><div className="v">{analytics.subscribed.toLocaleString()}</div><div className={`sub${analytics.subscribed ? " up" : ""}`}>{pctStr(analytics.subscribed, analytics.signups)} of signups</div></div>
          </div>
          {analytics.by_source.length > 0 && (
            <>
              <div className="ops-section-h"><h2>Signups by source</h2><span className="note">first-touch attribution · last {Math.round(analytics.window_secs / 86400)} days</span></div>
              <div className="plan-mix">
                {analytics.by_source.map((s) => (
                  <span key={s.source} className="plan-chip"><span className="pn">{s.source}</span><span className="pc">{s.signups}</span></span>
                ))}
              </div>
            </>
          )}
        </>
      )}

      {waitlist && waitlist.total > 0 && (
        <>
          <div className="ops-section-h">
            <h2>Pre-launch waitlist</h2>
            <span className="note">
              {waitlist.total.toLocaleString()} email{waitlist.total === 1 ? "" : "s"} captured
              {waitlist.pending > 0 ? ` · ${waitlist.pending} pending confirmation` : " · all confirmed"}
            </span>
          </div>
          <div style={{ display: "flex", gap: 8, flexWrap: "wrap", margin: "0 0 12px" }}>
            {waitlist.pending > 0 && (
              <button className="btn sm" disabled={busyId === "waitlist-resend"} onClick={resendConfirmations}>
                {busyId === "waitlist-resend" ? "Sending…" : `Resend confirmation to ${waitlist.pending} pending`}
              </button>
            )}
            <button className="btn sm" disabled={busyId === "waitlist-sync"} onClick={syncAudience}>
              {busyId === "waitlist-sync" ? "Syncing…" : "Sync to Resend audience"}
            </button>
          </div>
          <div className="bl-scroll" style={{ maxHeight: 260 }}>
            <table className="bl-table admin-table">
              <thead><tr><th>Email</th><th>Source</th><th>When</th></tr></thead>
              <tbody>
                {waitlist.recent.map((w, i) => (
                  <tr key={`${w.email}-${i}`}>
                    <td className="admin-email">{w.email}</td>
                    <td className="sm">{w.source ?? "direct"}</td>
                    <td><Time iso={w.created_at} /></td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </>
      )}

      {feedback && (
        <>
          <div className="ops-section-h">
            <h2>User feedback</h2>
            <span className="note">
              {feedback.items.length} total
              {feedback.open > 0 ? ` · ${feedback.open} new` : " · all triaged"}
            </span>
          </div>
          {feedback.items.length === 0 ? (
            <p className="muted" style={{ margin: "0 0 12px" }}>
              No feature requests or bug reports yet.
            </p>
          ) : (
            <div className="bl-scroll" style={{ maxHeight: 360 }}>
              <table className="bl-table admin-table">
                <thead>
                  <tr>
                    <th>Type</th>
                    <th>Title & details</th>
                    <th>From</th>
                    <th>Status</th>
                    <th>When</th>
                  </tr>
                </thead>
                <tbody>
                  {feedback.items.map((f) => (
                    <tr key={f.id}>
                      <td className="sm">{f.kind === "bug" ? "Bug" : "Feature"}</td>
                      <td style={{ maxWidth: 420 }}>
                        <div style={{ fontWeight: 600 }}>{f.title}</div>
                        <div className="sm muted" style={{ whiteSpace: "pre-wrap", marginTop: 2 }}>
                          {f.body}
                        </div>
                        {f.page_url && (
                          <div className="sm mono" style={{ marginTop: 4, opacity: 0.7, wordBreak: "break-all" }}>
                            {f.page_url}
                          </div>
                        )}
                      </td>
                      <td className="admin-email sm">{f.user_email}</td>
                      <td>
                        <select
                          value={f.status}
                          disabled={busyId === `fb-${f.id}`}
                          onChange={(e) => void changeFeedbackStatus(f.id, e.target.value)}
                          aria-label="Feedback status"
                        >
                          <option value="open">New</option>
                          <option value="planned">Planned</option>
                          <option value="in_progress">In progress</option>
                          <option value="done">Shipped</option>
                          <option value="declined">Not planned</option>
                        </select>
                      </td>
                      <td className="sm"><Time iso={f.created_at} /></td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          )}
        </>
      )}

      {nearLimit.length > 0 && (
        <>
          <div className="ops-section-h"><h2>Near plan limit</h2><span className="note">monitored backlinks vs plan cap, click to open</span></div>
          <div className="plan-mix">
            {nearLimit.slice(0, 8).map((u) => (
              <button key={u.id} className="plan-chip" onClick={() => { const o = orgs.find((x) => x.id === u.id); if (o) setDetail(o); }}>
                <span className="pn">{u.name}</span><span className="pc" style={u.pct != null && u.pct >= 100 ? { color: "var(--lost)" } : undefined}>{u.pct}%</span>
              </button>
            ))}
          </div>
        </>
      )}

      <div className="ops-section-h"><h2>Not yet instrumented</h2><span className="note">needs backend work, see docs/19</span></div>
      <div className="ops-gap">
        {[
          { t: "Provider & API usage / cost", d: "No external data provider is integrated; usage_records exists but is never populated (RecomputeUsage job has no handler). Needs metering." },
        ].map((o) => (
          <div className="ops-row" key={o.t}>
            <div className="ops-b"><div className="ops-t">{o.t}<span className="ops-pill">needs instrumentation</span></div><div className="ops-d">{o.d}</div></div>
          </div>
        ))}
      </div>

      {err && <p className="error">{err}</p>}

      <div className="panel">
        <div className="admin-toolbar">
          <div className="admin-tabs" role="tablist">
            <button role="tab" aria-selected={tab === "orgs"} className={tab === "orgs" ? "on" : ""} onClick={() => setTab("orgs")}>Workspaces <span className="n">{orgs.length}</span></button>
            <button role="tab" aria-selected={tab === "subscriptions"} className={tab === "subscriptions" ? "on" : ""} onClick={() => setTab("subscriptions")}>Subscriptions <span className="n">{subs.length}</span></button>
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
                      <button className="admin-name link" onClick={() => setDetail(o)} title="Open Customer 360">{o.name}</button>
                      <div className="admin-sub">/{o.slug}</div>
                    </td>
                    <td className="admin-email">{o.owner_email ?? "-"}</td>
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

        {tab === "subscriptions" && (
          <div className="bl-scroll">
            <table className="bl-table admin-table">
              <thead>
                <tr>
                  <th>Workspace</th>
                  <th>Owner</th>
                  <th>Plan</th>
                  <th>Status</th>
                  <th style={{ textAlign: "right" }}>MRR</th>
                  <th>Renews / Trial ends</th>
                </tr>
              </thead>
              <tbody>
                {fSubs.map((s) => {
                  const badge =
                    s.status === "active" ? "ok" :
                    s.status === "trialing" ? "info" :
                    s.status === "past_due" ? "bad" : "";
                  const renewIso = s.status === "trialing" ? s.trial_ends_at : s.current_period_end;
                  return (
                    <tr key={s.id}>
                      <td>
                        <button className="admin-name link" onClick={() => { const o = orgs.find((x) => x.id === s.id); if (o) setDetail(o); }} title="Open Customer 360">{s.name}</button>
                      </td>
                      <td className="admin-email">{s.owner_email ?? "-"}</td>
                      <td>{PLAN_LABEL[s.plan] ?? s.plan}</td>
                      <td style={{ whiteSpace: "nowrap" }}>
                        <span className={`abadge ${badge}`}>{s.status.replace("_", " ")}</span>
                        {s.comped && <span className="abadge info" style={{ marginLeft: 6 }}>comped</span>}
                        {s.cancel_at_period_end && <span className="abadge bad" style={{ marginLeft: 6 }}>cancels</span>}
                      </td>
                      <td className="mono" style={{ textAlign: "right" }}>{s.mrr_cents > 0 ? `$${Math.round(s.mrr_cents / 100).toLocaleString()}` : "-"}</td>
                      <td>{renewIso ? <Time iso={renewIso} /> : "-"}</td>
                    </tr>
                  );
                })}
                {fSubs.length === 0 && (
                  <tr>
                    <td colSpan={6} className="muted">
                      {term ? "No subscriptions match your search." : "No subscriptions yet."}
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
                    <td>{u.email_verified ? <span className="abadge ok">verified</span> : <span className="muted">-</span>}</td>
                    <td>{u.is_platform_admin ? <span className="abadge info">admin</span> : <span className="muted">-</span>}</td>
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
                    <td className="admin-email">{a.actor_email ?? "-"}</td>
                    <td><span className="atag">{a.action}</span></td>
                    <td className="sm">{a.org_name ?? a.resource_type ?? "-"}</td>
                    <td className="mono muted" style={{ maxWidth: 280, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }} title={a.after ?? undefined}>{a.after ?? "-"}</td>
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

      {detail && (
        <div className="drawer-backdrop" onClick={() => setDetail(null)}>
          <aside className="drawer" onClick={(e) => e.stopPropagation()}>
            <div className="panel-head">
              <h2 style={{ fontSize: 16 }}>Customer 360</h2>
              <button className="btn" onClick={() => setDetail(null)}>Close</button>
            </div>
            <div className="c360-head">
              <h2>{detail.name}</h2>
              {detail.suspended ? <span className="abadge bad">suspended</span> : <span className="abadge ok">active</span>}
            </div>
            <div className="c360-sub">/{detail.slug} <IdChip id={detail.id} /></div>
            <dl className="detail-grid">
              <div><dt>Owner</dt><dd className="mono">{detail.owner_email ?? "-"}</dd></div>
              <div><dt>Plan</dt><dd>{PLAN_LABEL[detail.plan] ?? detail.plan}</dd></div>
              <div><dt>Created</dt><dd><Time iso={detail.created_at} /></dd></div>
              <div><dt>Members</dt><dd className="mono">{detail.members}</dd></div>
              <div><dt>Projects</dt><dd className="mono">{detail.projects}</dd></div>
              <div><dt>Backlinks monitored</dt><dd className="mono">{detail.backlinks.toLocaleString()}</dd></div>
            </dl>
            {(() => {
              const sb = subs.find((s) => s.id === detail.id);
              if (!sb) return null;
              return (
                <>
                  <div className="c360-subhead">Subscription</div>
                  <dl className="detail-grid">
                    <div><dt>Status</dt><dd>{sb.status.replace("_", " ")}{sb.comped ? " · comped" : ""}</dd></div>
                    <div><dt>MRR</dt><dd className="mono">{sb.mrr_cents > 0 ? `$${Math.round(sb.mrr_cents / 100).toLocaleString()}/mo` : "-"}</dd></div>
                    {sb.status === "trialing" && sb.trial_ends_at && (
                      <div><dt>Trial ends</dt><dd><Time iso={sb.trial_ends_at} /></dd></div>
                    )}
                    {sb.current_period_end && (
                      <div><dt>{sb.cancel_at_period_end ? "Cancels" : "Renews"}</dt><dd><Time iso={sb.current_period_end} /></dd></div>
                    )}
                    {sb.canceled_at && (
                      <div><dt>Canceled</dt><dd><Time iso={sb.canceled_at} /></dd></div>
                    )}
                  </dl>
                </>
              );
            })()}
            <div className="c360-actions">
              <select
                value={detail.plan}
                disabled={busyId === detail.id}
                onChange={(e) => { const v = e.target.value; void changePlan(detail, v); setDetail((d) => (d ? { ...d, plan: v } : d)); }}
                aria-label="Change plan"
              >
                {ADMIN_PLANS.map((p) => (<option key={p} value={p}>{PLAN_LABEL[p] ?? p}</option>))}
              </select>
              <button className="btn" disabled={busyId === detail.id} onClick={() => { void toggleSuspend(detail); setDetail(null); }}>
                {detail.suspended ? "Unsuspend" : "Suspend"}
              </button>
              <button className="btn danger" disabled={busyId === detail.id} onClick={() => { void removeOrg(detail); setDetail(null); }}>
                Delete workspace
              </button>
            </div>
            <div className="c360-note">
              <b>Deeper telemetry for this workspace</b>, per-workspace check volume and failed jobs aren&rsquo;t exposed yet. The data exists (jobs, backlink_checks); surfacing it per workspace needs the admin read endpoints in docs/19.
            </div>
          </aside>
        </div>
      )}
    </div>
  );
}
